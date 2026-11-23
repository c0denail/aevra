# Görünür araştırma tarayıcısı

Araştırma alanında konu yazmak yeterlidir. Mevcut yerel planlayıcı yeni bir araştırma gerekip gerekmediğini belirler; araştırma gerekirse sağdaki tarayıcı sorguyu ve açılan gerçek sayfaları gösterir. Önceki kaynaklarla cevaplanabilen takip sorularında gereksiz yeni arama yapılmaz. Panel küçültülebilir; araştırma arka planda aynı tarayıcıda sürer. Kaynak kartları kaydedilmiş sayfayı uygulama içinde yeniden açar.

Varsayılan arama motoru Google'dır; sorgu `https://www.google.com/search` adresine, Türkçe arama arayüzü tercihiyle gider. Google sonuçlarındaki başlık bağlantıları ve güvenli hedef yönlendirmeleri okunur. Arama kutusunun textarea biçimi desteklenir. Google'ın metin aramasındaki boş Lens dosya alanı metin sorgusunu engellemez; seçilmiş dosya varsa otomatik form gönderimi engellenir. Doğrulama/olağan dışı trafik ve Google onay ekranları araştırma kaynağı sayılmaz, kullanıcıya kontrol bırakılır. Google Yardım/Geliştirici gibi normal belge alt alanları sonuç sayfasıyla karıştırılmaz. Doğrulama ekranı çevresinden dolaşılmaz veya sessizce başka sağlayıcıya geçilmez.

Adres çubuğuyla elle bir HTTPS sitesi açılabilir. “Bu sayfayı özetle” açık sayfayı okur; “Bu sitede … ara” algılanabilen site arama alanını kullanabilir. Aevra'nın her adımı kullanıcı tarafından tarif edilmez. Kontrolü al, mevcut üretimi iptal eder; toplanan kaynaklar ve varsa kısmi yanıt mevcut geçmiş kurallarıyla saklanır. Devam etmek için yeni bir sohbet mesajı yazılır. Bu, durdurulan işin otomatik devam ettirilmesi değildir.

## Uygulama

- `src/ResearchBrowser.tsx`: panel, adres ve gezinme kontrolleri, işlem listesi, boyut/konum bildirimleri. Ayar penceresi açıldığında, panel kapandığında veya araştırma devre dışıyken yerel tarayıcı görünümü gizlenir.
- `src-tauri/src/browser.rs`: ayrı `research-browser` WebView, yaşam döngüsü, gezinme, sabit DOM komutları ve sınırlı araştırma döngüsü. Tauri'nin çoklu WebView için `unstable` özelliği sabitlenmiş 2.12.5 sürümünde kullanılır.
- `src-tauri/src/browser_dom.js`: ana belgedeki metni, görünen bağlantıları ve arama alanlarını çıkaran sabit işlev. Girdi değerleri, parola alanları, metin kutuları ve betikler kanıt metnine alınmaz. Model JavaScript üretemez/çalıştıramaz.
- `crates/aevra-core/src/browser.rs`: Tauri'den bağımsız sayfa/eylem tipleri, şemalı yerel karar, URL ve sorgu sınırları. Model yalnız sağlanan bağlantı/arama alanı indeksini seçebilir. Son yanıt mevcut kaynaklı sohbet istemiyle hazırlanır; model, hafıza ve SQLite sistemi korunur.

En fazla 12 otomatik karar turu, 3 farklı arama ve ayara göre 1–5 kaynak vardır. Sayfa metni 12.000, kaynak alıntısı 3.200 karakter; bağlantılar 40, arama alanları 8 ile sınırlıdır. Araştırma döngüsü 180 saniyeyle sınırlıdır; modelin ilk hazırlanması buna dahil değildir. Sayfa bekleme 20, DOM cevabı 4, tek karar 40 saniyedir. Süre dolduğunda okunmuş kaynak varsa bunlarla yanıt hazırlanır; hiç kanıt yoksa görünür hata verilir.

## Sınırlar

Tarayıcı gerçek sayfa JavaScript'ini ve web isteklerini çalıştırır; bu nedenle eski basit HTTP okuyucunun 1 MB/DNS sabitleme/çerezsiz çalışma sınırları bu tarayıcı için geçerli değildir. Ayrı InPrivate profil kullanılır, ana uygulamanın tarayıcı depolaması paylaşılmaz. Site ve üçüncü taraf kaynaklar IP adresini, gönderilen sorguyu ve olağan tarayıcı bilgilerini görebilir. Ham sohbet geçmişi, dosya ekleri, model erişim anahtarı ve kişisel hafıza tarayıcıya verilmez. Son soru ve sayfa/kaynak başlığı kelimeleri sorguda kullanılabilir.

Uzak sayfaların Aevra Tauri komutlarına erişimi merkezi komut yönlendiricisinde engellenir; yetenek izinleri yalnız `main` WebView'a aittir. HTTPS/443, kimlik bilgisi içermeyen genel alan adı şartı üst düzey gezinmede uygulanır; IP, yerel ad, dosya/özel şema reddedilir. WebView2 alt kaynakları ve DNS kendi motoruyla yönetir; bunun tüm ağ trafiği için özel bir güvenlik duvarı veya DNS sabitlemesi olduğu iddia edilmez. Kamera, mikrofon, konum, pano izni, indirme ve yeni ayrı pencereler kapalıdır. Uygun HTTPS yeni pencere bağlantısı aynı panelde açılır.

Otomatik form işlemi yalnız tanınan arama alanlarına yöneliktir; hesap, parola, e-posta ve seçilmiş dosya içeren formlar kullanılmaz. Genel dosya yükleme formları desteklenmez; Google metin aramasındaki boş Lens alanı yukarıda belirtilen dar istisnadır. GET aramaları ile `/search`, `/arama` veya DuckDuckGo arama yollarındaki POST formları desteklenir; aynı köken şartı uygulanır. Ödeme, mesaj gönderme, yükleme veya genel düğme tıklama aracı yoktur. CAPTCHA çözülmez; kullanıcı tarayıcıdan kontrolü devralabilir. Iframe/shadow DOM arama alanları, tüm siteler, erişim kısıtları ve her sitenin özel arama bileşeni için çalışma garantisi yoktur.

Tarayıcı adımları oturum belleğindedir; kaynak alıntıları/URL'leri sohbetle SQLite'a kaydedilir. Tarayıcı verilerini temizle düğmesi ayrı tarayıcı verilerini temizleyip görünümü kapatır; sohbetleri silmez. Web araştırmasını kapatmak tarayıcıyı kapatır, kayıtlı kaynaklarla yerel sohbeti korur.

Araştırma modundan sohbet/çeviri/matematik/kodlamaya geçilince uzak tarayıcı görünümü kapatılır. Paneli yalnızca küçültmek oturumu kapatmaz.
