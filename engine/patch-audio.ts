const synthesize = "function K6(n,e=0,t){";
const synthesizeSteps = "function J6(n,e=0,t){";
const filter = "function kt(n,e,t,i,s){let[o,r,a,l,c]=mu(e,t,i,s),u=0,f=0;";
const yieldForWorker = "performance.now()-ie>6)break}l+=performance.now()-ie";
const finalize = "function j6(n,e,t){let i=Array.isArray(e)?e:[e];";

export const audioChanges: [string, string][] = [
  [
    synthesize,
    `${synthesize}
const bank=Ir[n],audio=saltlineKernels?.audio;
if(bank&&audio){
  const samples=audio.synthesize(bank,Gv(n),e%bank.n,t||bank.sr);
  if(samples)return samples;
}`,
  ],
  [
    synthesizeSteps,
    `${synthesizeSteps}
const bank=Ir[n],audio=saltlineKernels?.audio;
if(bank&&audio){
  const id=Gv(n),variant=e%bank.n,rate=t||bank.sr;
  if(audio.hasSound(id,variant,rate))return audio.steps(bank,id,variant,rate);
}`,
  ],
  [yieldForWorker, `(!pe&&ae.steps.pending)||${yieldForWorker}`],
  [
    filter,
    `${filter}
if(saltlineKernels?.audio.filter(n,o,r,a,l,c))return n;`,
  ],
  [
    finalize,
    `${finalize}
const nativeAudio=saltlineKernels?.audio.finalize(n,i,t);
if(nativeAudio)return nativeAudio;`,
  ],
];
