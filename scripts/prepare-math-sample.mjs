// Exercise the exact frontend converter without a browser or network request.
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import ts from 'typescript';
await mkdir('tmp/pdfs',{recursive:true});
await writeFile('tmp/pdfs/mathPdf.mjs',ts.transpileModule(await readFile('src/mathPdf.ts','utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText);
const {prepareMathAssets}=await import('../tmp/pdfs/mathPdf.mjs');
const text=String.raw`# Denklem, kesir ve kök
İkinci derece denklem: $ax^2+bx+c=0$.
Kökler şu formülle bulunur:
$$x=\frac{-b\pm\sqrt{b^2-4ac}}{2a}$$
Örneğin $x^2-5x+6=0$ için $x_1=2$ ve $x_2=3$.

## İntegral ve toplam
$$\int_0^1 x^2\,dx=\frac{1}{3}$$
$$\sum_{k=1}^{n} k=\frac{n(n+1)}{2}$$
Satır içinde $\frac{a}{b}$ kesri ve $\sqrt{2}$ kökü de düzgün hizalanır.

## Matris ve Yunan harfleri
$$A=\begin{pmatrix}1&2\\3&4\end{pmatrix},\quad \det A=-2$$
$$\alpha^2+\beta^2=\gamma^2,\qquad E=mc^2$$
Formüller yerel olarak dizilir. Türkçe: çığ, öğüt, şeker, İstanbul.
Almanca: Überprüfung, Größe, Straße.

`;
// Deliberate second page checks pagination with both inline and display formulas.
const content=text+'\n'+Array.from({length:22},(_,i)=>`Doğrulama satırı ${i+1}: $a_{${i+1}}=${i+2}$ ve kısa bir açıklama.`).join('\n')+'\n$$\\lim_{x\\to 0}\\frac{\\sin x}{x}=1$$\nSon kontrol: AEVRA-MATH-END-9264.\n\n## Kod korunur\n```js\nconst example = "$this_is_code$";\n```';
const assets=await prepareMathAssets(content);
await writeFile('tmp/pdfs/math-sample.json',JSON.stringify({text:content,assets}));
console.log(`${assets.length} formül uygulamanın dönüştürücüsüyle hazırlandı.`);
