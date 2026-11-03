export function download(name,mime,bytes){const url=URL.createObjectURL(new Blob([bytes],{type:mime}));const link=document.createElement("a");link.href=url;link.download=name;link.click();setTimeout(()=>URL.revokeObjectURL(url),10000);}
export function report_status(json){window.__commonsStatus=JSON.parse(json);window.dispatchEvent(new CustomEvent("commons-status",{detail:window.__commonsStatus}));}
// Pixel art already uses egui's edge antialiasing. A multisampled browser surface
// adds GPU work without improving its nearest-neighbor sprites and floor tiles.
export function prepare_canvas(canvas){
 const options={alpha:false,antialias:false,depth:false,stencil:false,preserveDrawingBuffer:false,powerPreference:"high-performance"};
 const gl=canvas.getContext("webgl2",options)||canvas.getContext("webgl",options);
 if(!gl)throw Error("This browser could not start WebGL. Enable graphics acceleration or try another browser.");
 const accepted=gl.getContextAttributes();window.__commonsGraphics={antialias:accepted.antialias,alpha:accepted.alpha,depth:accepted.depth,stencil:accepted.stencil};
}
