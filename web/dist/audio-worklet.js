class CommonsPCM extends AudioWorkletProcessor {
 constructor(){super();this.phase=0;this.sum=0;this.count=0;this.samples=[];}
 process(inputs){const input=inputs[0]?.[0];if(input)for(const value of input){this.sum+=value;this.count++;this.phase+=16000;if(this.phase>=sampleRate){this.phase-=sampleRate;this.samples.push(Math.round(Math.max(-1,Math.min(1,this.sum/this.count))*32767));this.sum=0;this.count=0;if(this.samples.length===320){const pcm=new Int16Array(this.samples);this.port.postMessage(pcm.buffer,[pcm.buffer]);this.samples=[];}}}return true;}
}
registerProcessor("commons-pcm",CommonsPCM);
