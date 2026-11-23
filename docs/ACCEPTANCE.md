# Kabul testleri

## Otomatik

- `pnpm build` ve `pnpm test`.
- `cargo fmt --all --check`.
- `cargo test -p aevra-core --locked`.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- `pnpm assets:verify`.
- Model varsa `cargo run -p aevra-core --example smoke`; `--web` yalnızca açık ağ denemesi için.
- Gerçek uyumlu GPU varsa `cargo run --release -p aevra-core --example hardware`: GPU/CPU/Otomatik motor geçişleri ve yeni modların üretim/kayıt denemesi. Normal CI'a dahil değildir; kodu yürütmez veya genel model kalitesini ölçmez.
- Gerçek model/projector/PDFium varsa `cargo run --release -p aevra-core --example files`: PNG ve taranmış PDF renk sorusu, metinli PDF/DOCX/ZIP'ten bilinen kod çıkarma, dört görsel, CPU görsel üretimi, eklerin SQLite ve biçim-2 yedek kaydı. İnternet kullanmaz; normal CI'da koşmaz.
- `cargo run --release -p aevra-core --example repairs`: Almanca/Türkçe, düşük token bütçesinde otomatik devam, 8.000 karakterden uzun tam PDF çevirisi ve PDF üretimi. Çıktı PDF'sini bağımsız okuyucuda sayfa sayfa kontrol edin.

## Windows uygulaması

1. Uygulamayı açın. Mesaj göndermeden modelin otomatik hazırlanıp hazır durumuna geçmesini bekleyin. Sohbette Türkçe soruya gerçek model yanıtı görünün.
2. İkinci mesajda ilk konuşmanın bağlamı kullanılsın. Uygulamayı kapatıp açın; konuşma ve sırası korunsun.
3. Çeviri modunda İngilizce metni Türkçeye çevirin. Ağ kapalıyken de çalışsın.
4. Yanıtı durdurun. Kullanıcı mesajı ve kısmi yanıt geçmişte kalsın. Gönderilen metin yazı alanından hemen temizlensin; yanıt sırasında yazılan yeni taslak korunsun. Model sonraki mesaj için hazır kalsın.
5. Araştırmada soruyu gönderince ek onay olmadan gerçek kaynak kartları ve numaralı alıntılar görünsün. Ayarlardan web araştırması kapalıysa dış istek gitmesin.
6. Araştırma ekranının sağında gerçek sayfalar görünsün. Normal bir konu yazmak arama ve kaynak toplamayı otomatik başlatsın. Kaynak düğmesi sayfayı uygulama içinde açsın. Panel küçültülünce araştırma sürsün; ayar penceresinde native görünüm gizlensin. Kontrolü al mevcut işi durdursun; adres çubuğu/geri/ileri/yenile kullanılabilsin. Elle açılan sayfa “Bu sayfayı incele” ile okunabilsin, desteklenen site arama alanı kullanılsın. Araştırma modundan çıkınca uzak görünüm kapansın. Modelin son yazılı yanıtındaki keyfî bağlantı veya görsel kendiliğinden yüklenmesin.
7. Modeli durdurun/eksik dosyayla açın; anlaşılır hata çıksın. Bozuk hashli model/motor çalıştırılmasın.
8. Konuşma silme onayı verildiğinde ilgili mesajlar da silinsin; başka konuşmalar kalsın.
9. Pencere 820×620 ve 1240×820 boyutlarında kullanılabilir olsun. Odak, Enter/Shift+Enter, Escape ve modal içi Tab davranışı çalışsın.
10. Model ve çeviri ayarlarında CPU, GPU ve Otomatik seçin; modelin yeniden hazırlanmasını ve gerçek arka ucun durumunu doğrulayın. CPU açıkça GPU katmanı kullanmasın. GPU başarısızsa önceki ayar geri yüklensin; Otomatik başarısız GPU'da CPU'ya dönsün. Yanıt sırasında donanım değişimi engellensin.
11. Matematik modunda bir denklem gönderin; formüller çevrimdışı gösterilsin. Kodlama modunda dil belirterek kod isteyin; kod blokları ve geçmiş korunsun. Hiçbir kod kendiliğinden çalıştırılmasın. v2 geçmiş/arşiv/kesilmiş yanıtlar v3'e geçince korunsun.
12. Ataç ile PNG/PDF/DOCX/DOC/TXT/kod/ZIP seçin. Kartta ad, tür, boyut, sayfa/kayıt ve uyarılar görünsün; ek kaldırılabilsin. Yalnızca ek gönderildiğinde varsayılan inceleme/çeviri isteği kullanılsın. Yanıt durdurulup uygulama yeniden açıldığında ekler korunsun. Yeni ek olmadan takip sorusunda önceki ek kullanılabilsin.
13. 20 MB üstü, bozuk/şifreli dosya, desteklenmeyen tür, tehlikeli yol, sembolik bağlantı, iç içe ZIP ve aşırı sıkıştırma davranışlarını doğrulayın. Araştırmada ek düğmesi kapalı olsun; dosya içeriği dış hizmete gitmesin. v3→v4 geçişinde mevcut geçmiş korunsun. 100 sayfa ve çeviri/sohbet bölüm seçimi sınırları ek kartı/belgelerde doğru anlatılsın.
14. Çeviride Almanca seçin; İngilizce metin Almancaya, Almanca metin Türkçe seçildiğinde Türkçeye dönsün. Uzun PDF'yi çevirin; son sayfa/kontrol ifadesi atlanmasın, ilerleme gösterilsin. Çeviri bütün bölümleri, diğer alanlar sınırlı bağlamı kullanır. Taranmış sayfalar çeviride sırayla işlenir; 100 sayfa/128 hazırlanmış görsel/16 MB hazırlanan görsel toplamı sınırlarını doğrulayın.
15. Uzun yanıtı 256/2048/8192 token parça bütçelerinde deneyin; `length` olduğunda otomatik devam etsin. İptal yeni parça başlatmasın; sınıra ulaşma `limited` görünsün ve tamamlandı sayılmasın. Sohbet, matematik, araştırma, kodlama ve çeviri ortak üretim yolunu kullanır.
16. Yanıt altındaki PDF düğmesiyle dosya kaydedin, iptal edin ve mevcut dosya üzerine kaydı deneyin. Türkçe/Almanca, çok sayfa, uzun satır/kod ve kaynaklar okunabilsin. Kısmi yanıt etiketi korunsun; desteklenmeyen yazı sisteminde anlaşılır hata çıksın. Dosya penceresi ve temiz Windows denemesi ayrıca kaydedilsin.

## Temiz Windows makinesi — genel dağıtım kapısı

Bu bölüm mevcut geliştirici bilgisayarında test edilmiş sayılmaz. Windows 10 22H2 ve Windows 11 x64 temiz VM üzerinde:

- Node, Rust, Python, Ollama, Cline, Visual C++ runtime ve WebView2 olmayan ortamı hazırlayın.
- İnterneti kapatın; dağıtım ZIP'ini açıp EXE ve `.bin` dosyaları yan yana dururken Setup EXE'yi çalıştırın. Haricî indirme olmadan WebView2 ve Aevra kurulabilsin.
- İlk açılışta model hazır olsun; çevrimdışı sohbet ve çeviri tamamlanabilsin.
- RAM/disk tüketimi, CPU uyumluluğu ve ilk token gecikmesi kaydedilsin.
- Vulkan sürücüsü olmayan temiz VM'de Otomatik → CPU; uyumlu GPU'lu sistemlerde VRAM, sürücü uyumluluğu ve açık GPU başarısızlık davranışı ayrıca sınansın.
- Güncelleme üzerine kurulumda geçmiş korunsun. Kaldırma uygulama/model dosyalarını kaldırsın, kullanıcı geçmişini korusun.
- İmzalı EXE ve kurulum için yayıncı, bütünlük, lisans dosyaları ve SmartScreen davranışı doğrulansın.

## 9B / araştırma sohbeti / matematikli belgeler

- Manifest ve kuruluma giren dosya Qwen3.5-9B Q4_K_M olmalı; eski 4B önbelleği paketlenmemeli. GPU auto-fit katmanları 16.384 bağlamı korumalı.
- Araştırma sonucu üzerine “bunu basitleştir” sorusu aynı konudan devam etmeli; yeni karşılaştırma yeni arama planlamalı. Ham özel geçmiş terimleri sorguya girmemeli. Web kapalıyken takip sohbeti çalışmalı.
- PDF kesir/kök/integral/toplam/matris ve satır içi formülleri kod metni yerine dizilmiş biçimde göstermeli. Her sayfa bağımsız okuyucuda denetlenmeli; yeniden yüklenince formüller ve son kontrol okunmalı.
- DOCX standart Office Math kesri/kökü LaTeX olarak alınmalı; şifresiz Word 97–2003 DOC okunmalı, şifreli DOC reddedilmeli. Formüllü PDF çevirisinde metinli sayfa görseli ikinci çeviri görevi oluşturmamalı.

## Otomatik kişisel hafıza kabulü

- Yeni mesajdaki açık kişisel bilgi kaydolmalı; yeni konuşma ilgili kaydı kullanmalı. Model ağırlığı değişmemeli ve ağ isteği olmamalı.
- A2→B1 düzeltmesi tek kaydı güncellemeli. Tek seferlik kısa yanıt isteği pending olmalı; aynı tercih üç farklı konuşmada etkinleşmeli.
- Düzenle/unut/tümünü temizle/yeniden izin ver ve on dakikalık undo sınanmalı; eski çıkarım düzenlenmiş/silinmiş değeri geri yazmamalı. Silinen aynı seviye bilgisi genel learning_goal alanı üzerinden geri gelmemeli.
- Hafızasız konuşma işareti yeniden açılma ve sohbet yedeğinde korunmalı; global kapalıyken ve bu sohbette kayıtlar okunmamalı/yazılmamalı. Geçmiş kaydının sürdüğü açıkça görünmeli.
- Alıntı, kod, ek, çeviri ve belirgin parola/API anahtarı/iletişim bilgisi çıkarılmamalı. Araştırma sorgusuna hafıza eklenmemeli. Bozuk/kesilmiş JSON kayıt üretmemeli.
- Arka plan çıkarımında yeni mesaj öncelik almalı; iptal kalan kuyruğu korumalı. Yeniden başlatma işi sürdürmeli; unutma veya kapatma eski işleri geçersiz kılmalı.
- Gerçek 9B örneği cargo run --release -p aevra-core --example memory; normal CI dışıdır, internet kullanmaz. Yerel Windows UI'da gerçek bildirim/geri alma/dosya seçiciler ve temiz VM ayrıca kabul edilir.
