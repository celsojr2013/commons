const {chromium}=require('playwright');
const {spawn}=require('node:child_process');
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),assert=require('node:assert/strict');
const root=path.resolve(__dirname,'../..');
const folder=fs.mkdtempSync(path.join(os.tmpdir(),'commons-features-'));
const errors=[],bots=[];let server,browser,first,second;
const password=process.env.COMMONS_TEST_PASSWORD||'qa-join-only',adminPassword=process.env.COMMONS_TEST_ADMIN_PASSWORD||'qa-admin-only';
const started=Date.now(),log=(...a)=>console.log(Math.round((Date.now()-started)/1000)+'s',...a);
const output=path.join(root,'shots/features');fs.mkdirSync(output,{recursive:true});fs.mkdirSync(path.join(root,'data'),{recursive:true});
async function press(page,id){
 await page.waitForFunction(id=>{const s=window.__commonsStatus,r=s?.controls?.[id];if(!r)return false;window.__stable??={};const old=window.__stable[id],key=r.join(',');if(old?.frame===s.frame)return old.count>=3;const count=old?.key===key?old.count+1:0;window.__stable[id]={key,frame:s.frame,count};return count>=3;},id,{timeout:15000});
 const r=await page.evaluate(id=>window.__commonsStatus.controls[id],id);
 await page.mouse.click(r[0]+r[2]/2,r[1]+r[3]/2,{delay:45});
 await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
}
async function profile(page){if(!await page.evaluate(()=>window.__commonsStatus.profile_open))await press(page,'profile');}
async function fillControl(page,id,text){await press(page,id);await page.keyboard.press(process.platform==='darwin'?'Meta+A':'Control+A');await page.keyboard.type(text,{delay:12});}
async function join(context,url,name){
 const page=await context.newPage();page.on('pageerror',e=>errors.push(e.message));
 await page.addInitScript(()=>{
  const Native=WebSocket;window.__events=[];
  window.WebSocket=class extends Native{constructor(...args){super(...args);this.addEventListener('message',m=>{try{const e=JSON.parse(m.data);window.__events.push(e);if(window.__events.length>300)window.__events.shift();}catch{}});}};
  // Synthetic screen only: exercises the real encoder/transport without capturing the person's desktop.
  navigator.mediaDevices.getDisplayMedia=async()=>{
   const c=document.createElement('canvas');c.width=1920;c.height=1080;const p=c.getContext('2d');
   const draw=()=>{p.fillStyle='#172235';p.fillRect(0,0,c.width,c.height);p.fillStyle='#e9f1ff';p.font='70px sans-serif';p.fillText('Commons · Shared document',90,120);p.font='32px sans-serif';p.fillText('Synthetic screen for independent webcam + screen validation',90,205);p.fillStyle='#6cd6b5';p.fillRect(90+(Date.now()%400),320,700,280);};draw();
   const timer=setInterval(draw,200);const stream=c.captureStream(5),track=stream.getVideoTracks()[0];
   window.__qaScreenTrack=track;track.addEventListener('ended',()=>clearInterval(timer));return stream;
  };
 });
 await page.goto(url+'/?qa=1');await page.locator('#name').fill(name);await page.locator('#password').fill(password);await page.locator('#enter').click();
 await page.waitForFunction(()=>window.__commonsStatus?.connected,{timeout:30000});return page;
}
async function start(){
 if(process.env.COMMONS_TEST_URL)return process.env.COMMONS_TEST_URL;
 server=spawn(path.join(root,'target/debug/commons-desktop'),['--serve-web','--http-bind','127.0.0.1:0','--tcp-bind','127.0.0.1:0','--name','Feature validation'],{cwd:root,env:{...process.env,COMMONS_DATA_DIR:folder,COMMONS_PASSWORD:password,COMMONS_ADMIN_PASSWORD:adminPassword}});
 return await new Promise((resolve,reject)=>{let text='';const timeout=setTimeout(()=>reject(Error('Server readiness deadline')),15000);server.stdout.on('data',b=>{text+=b;const m=text.match(/Browser: (http:\/\/localhost:\d+)/);if(m){clearTimeout(timeout);resolve(m[1]);}});server.stderr.on('data',b=>process.stderr.write(b));server.on('exit',c=>{clearTimeout(timeout);reject(Error('Server exited '+c));});});
}
(async()=>{try{
 const url=await start();log('Server ready',url);
 browser=await chromium.launch({...(process.env.COMMONS_TEST_BROWSER?{executablePath:process.env.COMMONS_TEST_BROWSER}:process.platform==='darwin'&&fs.existsSync('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome')?{executablePath:'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'}:{}),headless:true,args:['--use-angle=swiftshader','--enable-unsafe-swiftshader','--use-fake-device-for-media-stream','--use-fake-ui-for-media-stream','--disable-background-timer-throttling','--disable-renderer-backgrounding','--disable-backgrounding-occluded-windows']});
 const context=await browser.newContext({viewport:{width:1440,height:900},ignoreHTTPSErrors:!!process.env.COMMONS_TEST_URL,permissions:['camera','microphone']});
 first=await join(context,url,'QA Alice');second=await join(context,url,'QA Bob');
 await first.waitForFunction(()=>window.__commonsStatus.people===2);const initialMaps=await first.evaluate(()=>window.__commonsStatus.maps);assert(initialMaps>=1&&initialMaps<128);
 await first.bringToFront();await profile(first);await press(first,'administration');await fillControl(first,'admin_password',adminPassword);await press(first,'admin_login');
 await first.waitForFunction(()=>window.__commonsStatus.admin);log('Administrator authenticated');await fillControl(first,'new_room_name','Review studio');await press(first,'room_create');await first.waitForFunction(n=>window.__commonsStatus.maps===n,initialMaps+1);await second.waitForFunction(n=>window.__commonsStatus.maps===n,initialMaps+1);
 await first.screenshot({path:path.join(output,'01-admin.png')});
 // Close the egui window through its real title-bar close control.
 await first.keyboard.press('Escape');
 // Administration doesn't consume Escape; use the dialog close button's visible bounds from a QA control.
 await press(first,'admin_close');
 await press(first,'room'+initialMaps);await first.waitForFunction(n=>window.__commonsStatus.room===n,initialMaps);
 await press(first,'room_edit');await fillControl(first,'editor_name','Review revised');await press(first,'editor_save');
 await second.waitForFunction(()=>window.__events.some(e=>e.State?.rooms?.some(r=>r.name==='Review revised')));log('Web administrator edits and publishes map');
 await profile(first);await press(first,'administration');await press(first,'room_delete');await press(first,'room_delete_confirm');
 await first.waitForFunction(n=>window.__commonsStatus.maps===n&&window.__commonsStatus.room===0,initialMaps);await second.waitForFunction(n=>window.__commonsStatus.maps===n,initialMaps);
 await press(first,'admin_logout');await first.waitForFunction(()=>!window.__commonsStatus.admin);await press(first,'admin_close');log('Web administration, creation, occupied-map deletion, relocation and logout passed');
 await press(first,'camera');await second.bringToFront();await press(second,'call_toggle');await press(second,'meeting_open');
 await second.waitForFunction(()=>window.__commonsStatus.meeting&&window.__commonsStatus.remote_video===1);
 await first.bringToFront();await press(first,'screen_share');await first.waitForFunction(()=>window.__commonsStatus.screen_ready&&window.__commonsStatus.camera);
 await second.waitForFunction(()=>window.__commonsStatus.remote_screens===1&&window.__commonsStatus.remote_video===1&&window.__commonsStatus.pin.startsWith('Some(Screen'));
 assert.deepEqual(await first.evaluate(()=>window.__commonsStatus.local_screen_size),[1920,1080]);
 await second.screenshot({path:path.join(output,'02-shared-screen.png')});log('Webcam and readable screen arrive independently; screen is automatically pinned');
 await second.bringToFront();await press(second,'screen_share');await second.waitForFunction(()=>window.__commonsStatus.screen_ready);
 const oldPin=await first.evaluate(()=>window.__commonsStatus.pin);await first.waitForFunction(()=>!!window.__commonsStatus.controls.screen_pin_second);assert.equal(await first.evaluate(()=>window.__commonsStatus.pin),oldPin);
 await first.bringToFront();await press(first,'screen_pin_second');await first.waitForFunction(()=>window.__commonsStatus.remote_screens===1);await press(first,'camera_pin_first');await first.waitForFunction(()=>window.__commonsStatus.pin.startsWith('Some(Camera'));
 await first.screenshot({path:path.join(output,'03-camera-pin.png')});log('Two presenters, individual screen selection and camera pin passed');
 await press(first,'devices');await press(first,'quality_medium');await first.waitForFunction(()=>window.__commonsStatus.quality==='Medium'&&window.__commonsStatus.local_camera_size?.[0]===640);await press(first,'quality_high');await first.waitForFunction(()=>window.__commonsStatus.quality==='High'&&window.__commonsStatus.local_camera_size?.[0]===1280);await press(first,'quality_low');await press(first,'device_save');log('Camera quality changes while screen remains active');
 for(let i=0;i<6;i++){
  const bot=new WebSocket((process.env.COMMONS_TEST_WS_URL||url.replace('http','ws'))+'/ws');bots.push(bot);
  await new Promise((resolve,reject)=>{bot.onopen=()=>bot.send(JSON.stringify({Hello:{name:'QA visitor '+i,password,color:i,protocol:5}}));bot.onerror=reject;bot.onmessage=e=>{if(JSON.parse(e.data).Welcome)resolve();};});
 }
 await first.waitForFunction(()=>window.__commonsStatus.people===8);await press(first,'meeting_gallery');await press(first,'meeting_next');await first.waitForFunction(()=>window.__commonsStatus.meeting_page===1);await first.screenshot({path:path.join(output,'04-pagination.png')});await press(first,'meeting_previous');log('Eight participants and gallery pagination passed');
 await press(first,'meeting_map');await first.waitForFunction(()=>!window.__commonsStatus.meeting);assert.equal(await first.evaluate(()=>window.__commonsStatus.room),0);
 // Browser stop-sharing control ends screen capture without changing webcam state.
 await first.evaluate(()=>window.__qaScreenTrack.dispatchEvent(new Event('ended')));await first.waitForFunction(()=>!window.__commonsStatus.screen&&window.__commonsStatus.camera);log('Browser stop-sharing leaves webcam active');
 const report={url,initial_maps:initialMaps,original_maps_preserved:true,web_admin:true,create_delete_maps:true,occupied_map_relocation:true,admin_logout:true,independent_webcam_screen:true,multiple_presenters:true,individual_pin:true,camera_pin:true,quality_profiles:true,pagination:8,screen_picker:'synthetic getDisplayMedia; actual OS/browser picker requires human acceptance',errors};
 assert.deepEqual(errors,[]);fs.writeFileSync(path.join(root,'data/features-browser-check.json'),JSON.stringify(report,null,2));log('All feature browser checks passed');
 }catch(e){if(first){await first.screenshot({path:path.join(output,'failure.png')}).catch(()=>{});log('Status',await first.evaluate(()=>window.__commonsStatus).catch(()=>{}));}throw e;}
 finally{bots.forEach(b=>b.close());if(browser)await browser.close();if(server)server.kill('SIGINT');}
})().catch(e=>{console.error(e);process.exitCode=1;});
