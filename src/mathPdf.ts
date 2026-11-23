export interface MathAsset {tex:string;display:boolean;svg:string}
export interface MathPart {text:string;math?:boolean;display?:boolean}

export function mathParts(source:string):MathPart[]{
  const result:MathPart[]=[];let buffer='';let i=0;let fence=false;
  while(i<source.length){
    if(source.startsWith('```',i)){fence=!fence;buffer+='```';i+=3;continue;}
    if(!fence && source[i]==='`'){const end=source.indexOf('`',i+1);if(end>=0){buffer+=source.slice(i,end+1);i=end+1;continue;}}
    let open='';let close='';let display=false;
    if(!fence && (i===0||source[i-1]!=='\\')){
      if(source.startsWith('$$',i)){open='$$';close='$$';display=true;}
      else if(source.startsWith('\\[',i)){open='\\[';close='\\]';display=true;}
      else if(source.startsWith('\\(',i)){open='\\(';close='\\)';}
      else if(source[i]==='$'){open='$';close='$';}
    }
    if(open){const end=source.indexOf(close,i+open.length);if(end>=0){const text=source.slice(i+open.length,end);if(text.trim() && (display||!text.includes('\n'))){if(buffer)result.push({text:buffer});buffer='';result.push({text:text.trim(),math:true,display});i=end+close.length;continue;}}}
    buffer+=source[i++];
  }
  if(buffer)result.push({text:buffer});return result;
}
export function normalizeMath(source:string):string{return mathParts(source).map(p=>p.math?(p.display?`$$${p.text}$$`:`$${p.text}$`):p.text).join('');}
export async function prepareMathAssets(source:string):Promise<MathAsset[]>{
  const parts=mathParts(source).filter(p=>p.math);if(!parts.length)return[];
  if(parts.length>200||parts.some(p=>p.text.length>10000))throw new Error('PDF en fazla 200 formül ve formül başına 10.000 karakter destekler.');
  const [{mathjax},{TeX},{SVG},{liteAdaptor},{RegisterHTMLHandler}]=await Promise.all([
    import('mathjax-full/js/mathjax.js'),import('mathjax-full/js/input/tex.js'),import('mathjax-full/js/output/svg.js'),import('mathjax-full/js/adaptors/liteAdaptor.js'),import('mathjax-full/js/handlers/html.js'),import('mathjax-full/js/input/tex/AllPackages.js')
  ]);
  const adaptor=liteAdaptor();RegisterHTMLHandler(adaptor);
  const doc=mathjax.document('',{InputJax:new TeX({packages:['base','ams','newcommand','boldsymbol','mathtools'],maxBuffer:10000,maxMacros:1000}),OutputJax:new SVG({fontCache:'local'})});
  const result:MathAsset[]=[];
  for(const part of parts){if(result.some(p=>p.tex===part.text&&p.display===Boolean(part.display)))continue;
    const node=doc.convert(part.text,{display:Boolean(part.display)});const svg=adaptor.innerHTML(node);
    if(!svg.startsWith('<svg')||svg.includes('data-mjx-error'))throw new Error(`PDF formülü dizilemedi: ${part.text.slice(0,80)}`);
    result.push({tex:part.text,display:Boolean(part.display),svg});
  }return result;
}
