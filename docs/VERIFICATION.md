# Doğrulama kaydı — 17 Kasım 2026

## Kartlı çalışma alanı tasarımı

Kullanıcının görsel referansı Aevra'nın gerçek özelliklerine uyarlandı: beyaz/lavanta kartlar, ince arka plan ızgarası, mor ve turkuaz vurgular, kişisel alan başlığı, kartlı ana sayfa, hızlı başlangıçlar ve son konuşmalar. Açık/koyu ana pencere, sohbet/çeviri/araştırma ekranları ve ayar vurgu renkleri güncellendi. Sahte görev, toplantı, kişi, ekip veya ilerleme verisi eklenmedi. Tasarım kapsamı docs/DESIGN.md içindedir.

Ana sayfadan beş çalışma alanı açılır, taslaklar hazırlanır, gerçek konuşmalar seçilir ve ilgili ayar bölümlerine gidilir. Profil kartı kişiselleştirmeyi, model alt çubuğu model ayarlarını açar. Önceki gönderim/iptal, ek, PDF, hafıza, arşiv ve Google araştırma akışı korunur. Yeni Rust/SQLite şeması veya model değişikliği yoktur.

## Çalıştırılan kontroller

- pnpm build geçti. Vite'ın önceden bulunan büyük MathJax/parça uyarısı sürer; derleme hatası yoktur.
- pnpm test: 7 dosyada 37 arayüz testi geçti. Önceki 33 testin yanında gerçek tercihler/son konuşmalar, arşiv filtreleme, alan/starter seçimi, önerileri gizleme ve ana sayfadan konuşmaya/çalışma alanına geçiş sınandı. Başlangıç düğmesi taslak hazırlayıp kendiliğinden göndermez.
- pnpm test:ui geçti. Gerçek üretim React arayüzü ayrı headless Chrome profilinde 1240×820 ve 820×620 boyutlarında çalıştırıldı. Açık/koyu ana sayfa, Almanca hedef seçilen çeviri, araştırma paneli, paneli küçültme ve profil ayarına erişim denendi. Yatay taşma ve JavaScript çalışma hatası yok; mesaj kutusu görünür kaldı.
- output/ui/aevra-home-light.png, aevra-home-dark.png, aevra-home-compact.png, aevra-translate-compact.png ve aevra-research-compact.png gözle incelendi. Koyu kart gölgesi düzeltilip ekran görüntüleri yeniden üretildi. output/ui/report.json sonucu ok=true.
- Görsel deneme üretim ön yüzünü kullanır; yalnız tarayıcıya özgü önizleme bandı, masaüstü yerleşimini göstermek için testte gizlenir. Görseller native Windows pencere yakalaması değildir. Kullanıcı SQLite'ı ve gerçek model kullanılmaz, harici sayfa istekleri engellenir. Chrome/Playwright yalnız geliştirici kontrolüdür; son kullanıcı kurulumuna eklenmez.
- pnpm bundle:windows geçti: Windows üretim EXE'si, model/motor içeren taşınabilir klasör ve Inno Setup EXE + beş BIN diliminden tek çevrimdışı ZIP yeniden üretildi.
- pnpm package:verify geçti: kaynak ve taşınabilir klasördeki 92 varlık, 9B model listesi, ZIP özeti ve ZIP içindeki altı kurulum dosyasının SHA-256 değerleri doğrulandı.

## Güncel Windows dağıtımı

- Uygulama: artifacts/Aevra-memory/Aevra.exe, 22.955.008 bayt. Yanındaki resources klasörüyle kullanılır.
- EXE SHA-256: f92be0a2c02d7f3bd80945c9df5e8c0721352476b1b4b576c1604c09794177f8.
- ZIP: artifacts/Aevra-0.1.0-windows-x64-offline.zip, 7.352.743.728 bayt.
- ZIP SHA-256: 3caffcefae945115281e0c7ef3fee4b89c44fd49821adbb4cf312830d9eb48ae.
- Qwen3.5-9B Q4_K_M + eşleşen f16 projector, llama.cpp b9222 CPU/Vulkan ve SQLite v5 aynı sabit sürümlerdedir. Google varsayılan arama motorudur. Model/motor varlıkları ve kullanıcı verileri bu tasarım değişikliği için değiştirilmedi.

## Önceki doğrulamalar ve açık kapsam

15 Kasım sağlayıcı değişiminde 50 çekirdek testi, Clippy/format ve Google üzerinde gerçek WebView2 kutusu/sonuçları/kaynak okuma denemesi geçmişti. Ondan önce aynı görünür araştırma döngüsü gerçek 9B modeliyle iki resmî Rust sayfasını otomatik toplamıştı. Bu arayüz değişikliğinde çekirdek kodu değiştirilmedi; gerçek model/web/PDF/Word senaryoları yeniden çalıştırılmadı.

Temiz Windows 10/11 VM'de kurulum/kaldırma, native pencerede bütün etkileşimlerin elle görsel kabulü, farklı GPU'lar ve kod imzası tamamlanmış sayılmaz. Her site veya CAPTCHA için otomatik çalışma garantisi yoktur. SQLite/hafıza şifreleme ve sesli konuşma eklenmedi. Ağ/form sınırları docs/RESEARCH_BROWSER.md ve docs/SECURITY.md içindedir.
