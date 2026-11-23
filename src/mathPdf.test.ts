import {describe,it,expect} from 'vitest';
import {mathParts,normalizeMath,prepareMathAssets} from './mathPdf';
describe('Yerel PDF formülleri',()=>{
  it('matematik ayraçlarını korur ve kodu değiştirmez',()=>{
    const text=String.raw`Sonuç \(\frac{1}{2}\) ve \[x^2\]. Kod: `+'`$a$`';
    expect(mathParts(text).filter(p=>p.math)).toHaveLength(2);
    expect(normalizeMath(text)).toContain('$\\frac{1}{2}$');
    expect(normalizeMath(text)).toContain('`$a$`');
  });
  it('kesir ve kökü dış bağlantısız SVG olarak diziler',async()=>{
    const assets=await prepareMathAssets(String.raw`$$x=\frac{-b+\sqrt{b^2-4ac}}{2a}$$`);
    expect(assets).toHaveLength(1);expect(assets[0].svg).toContain('<path');expect(assets[0].svg).not.toContain('data-mjx-error');
    expect(await prepareMathAssets('Normal metin')).toEqual([]);
  });
});
