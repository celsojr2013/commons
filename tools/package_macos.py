from pathlib import Path
import shutil,subprocess,hashlib,json
app=Path(__file__).resolve().parents[1]
bundle=app/'dist/Commons.app'
exe=bundle/'Contents/MacOS/Commons'
exe.parent.mkdir(parents=True,exist_ok=True)
staged=exe.with_name('Commons.next')
shutil.copy2(app/'target/debug/commons-desktop',staged)
staged.replace(exe)
shutil.copy2(app/'platform/Info.plist',bundle/'Contents/Info.plist')
subprocess.run(['codesign','--force','--deep','--sign','-','--entitlements',str(app/'platform/entitlements.plist'),str(bundle)],check=True)
subprocess.run(['codesign','--verify','--deep','--strict',str(bundle)],check=True)
receipt={'bundle':'dist/Commons.app','sha256':hashlib.sha256(exe.read_bytes()).hexdigest(),'camera_and_microphone':'Off until user enables them; macOS permission descriptions included.','local_signing':'Ad-hoc signed and verified for this Mac.'}
(app/'data/macos-bundle.json').write_text(json.dumps(receipt,indent=2))
print('Packaged Commons.app; signature verified.')
