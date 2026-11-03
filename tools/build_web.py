"""Build the real Rust/egui WebAssembly client and its self-contained static package."""
from pathlib import Path
import subprocess,os,shutil,gzip,hashlib,json
ROOT=Path(__file__).resolve().parents[1]
env=os.environ.copy()
compiler_meta=ROOT/'.wasm-sysroot/compiler.json'
compiler=json.loads(compiler_meta.read_text()) if compiler_meta.exists() else {}
if (ROOT/'.wasm-sysroot/lib/rustlib/wasm32-unknown-unknown').exists() and Path(compiler.get('rustc','/nonexistent')).is_file():
 local_rustc=compiler['rustc']
 if subprocess.check_output([local_rustc,'--version'],text=True).strip()!=compiler['version']:
  raise SystemExit('The isolated WebAssembly standard library needs its recorded Rust version. Use a matching compiler or install the wasm32 target for your active Rust toolchain.')
 env['RUSTC']=local_rustc
 env['CARGO_ENCODED_RUSTFLAGS']='--sysroot\x1f'+str(ROOT/'.wasm-sysroot')
 linker=Path.home()/'.rustup/toolchains/stable-x86_64-apple-darwin/lib/rustlib/x86_64-apple-darwin/bin/rust-lld'
 if linker.exists():env['CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER']=str(linker)
subprocess.run(['cargo','build','--release','--target','wasm32-unknown-unknown','--lib'],cwd=ROOT,env=env,check=True)
bindgen=shutil.which('wasm-bindgen')
if not bindgen and __import__('platform').system()=='Darwin' and (ROOT/'tools/bin/wasm-bindgen').exists():bindgen=ROOT/'tools/bin/wasm-bindgen'
if not bindgen:raise SystemExit('Install wasm-bindgen-cli matching Cargo.lock, then run this script again.')
out=ROOT/'web/dist';out.mkdir(parents=True,exist_ok=True)
subprocess.run([str(bindgen),'--target','web','--no-typescript','--out-dir',str(out),'--out-name','commons',str(ROOT/'target/wasm32-unknown-unknown/release/commons_desktop.wasm')],check=True)
for name in ['index.html','entry.js','style.css','audio-worklet.js']:shutil.copy2(ROOT/'web'/name,out/name)
manifest=[]
for p in sorted(out.rglob('*')):
 if p.is_file() and p.suffix!='.gz':
  data=p.read_bytes();compressed=gzip.compress(data,compresslevel=9,mtime=0);p.with_suffix(p.suffix+'.gz').write_bytes(compressed)
  manifest.append({'file':str(p.relative_to(out)),'bytes':len(data),'gzip_bytes':len(compressed),'sha256':hashlib.sha256(data).hexdigest()})
(ROOT/'data/web-assets.json').write_text(json.dumps(manifest,indent=2))
print('Built browser client. Run cargo build again to embed it in the desktop/server package.')
