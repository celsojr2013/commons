import init,{WebHandle} from "/commons.js";
const form=document.querySelector("#join"),name=document.querySelector("#name"),password=document.querySelector("#password"),button=document.querySelector("#enter"),status=document.querySelector("#loading"),canvas=document.querySelector("#commons-canvas");
try{name.value=JSON.parse(localStorage.getItem("commons-v4/settings.json")||"{}").name||"";}catch{}
document.querySelector("#device-note").textContent=window.isSecureContext?"Camera and microphone stay off until you choose to use them.":"This link supports rooms and chat. For camera and microphone, ask your host for their HTTPS link.";
let loaded;
form.addEventListener("submit",async event=>{event.preventDefault();if(!name.value.trim())return;button.disabled=true;status.textContent="Opening your workspace…";try{
 loaded??=init();await loaded;const handle=new WebHandle();window.commons=handle;canvas.hidden=false;await handle.start(canvas,name.value.trim(),password.value);password.value="";document.querySelector("#welcome").hidden=true;canvas.focus();
}catch(error){canvas.hidden=true;button.disabled=false;status.textContent="Could not open the workspace. Reload and try again. "+error;console.error(error);loaded=undefined;}});
window.addEventListener("error",event=>{if(canvas.hidden)return;const fatal=document.querySelector("#fatal");fatal.hidden=false;fatal.textContent="The workspace stopped unexpectedly. Reload this page to reconnect. "+event.message;});
window.addEventListener("pagehide",()=>window.commons?.destroy());
