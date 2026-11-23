# Gizlilik ve güvenlik sınırları

## Uygulanan önlemler

- WebView'dan genel HTTP, kabuk veya dosya eklentilerine erişim yok. CSP yalnızca uygulama varlıkları ve Tauri iletişimine izin verir. Fontlar yereldir. Model yanıtındaki HTML çalıştırılmaz, uzak görseller yüklenmez.
- Yerel model yalnızca loopback üzerinde, rastgele port ve süreç başına anahtar ile çalışır. Anahtar arayüze, veritabanına veya günlüğe verilmez. Proxy ve otomatik yönlendirme yerel model istemcisinde kapalıdır.
- Motor, sabit dosya yoluyla ve argüman dizisiyle çalıştırılır. Shell kullanılmaz. Ortam temizlenir; dışarıdan LLAMA_ARG veya araç/MCP ayarı miras alınmaz. Araç çalıştırma, dosya araçları, web arayüzü ve dış sunucu yapılandırması sunulmaz.
- CPU/GPU tercihi yalnızca sabitlenmiş, hash doğrulanan paket motorları arasında seçim yapar. GPU cihaz kimliği motorun sınırlı cihaz listesinden alınır; kullanıcıdan çalıştırılabilir yol veya shell argümanı alınmaz. Matematik ve kodlama da çevrimdışıdır; formüller yerel KaTeX ile güvenilmeyen komutlara izin vermeden gösterilir. Üretilen kod yürütülmez.
- Windows Job Object, uygulama kapanırsa model sürecinin de kapanmasını sağlar. Kullanıcı iptalinde üretim isteği kesilir; model sonraki mesaj için bellekte kalır.
- Araştırmada yerel planlayıcı yeni arama gerekip gerekmediğini belirler. Sorgu yalnız güncel soru ve genel kaynak başlıklarının sözcüklerinden oluşturulur. Ek onay penceresi yoktur; araştırma ayarlardan kapatılabilir. Ham sohbet geçmişi sağlayıcıya gönderilmez; bağlam yalnız yerel planlayıcıda değerlendirilir. Web kapalıyken aramasız yerel konuşma sürer. Araştırmayı başlatmak, sağlayıcı ve bulunan sayfalara belirtilen sınırlarda bağlantı iznidir.
- Masaüstü araştırması ayrı, InPrivate WebView2 tarayıcısında görünür şekilde çalışır. Ana uygulamanın yetenekleri yalnız `main` WebView'a aittir; özel uygulama komutları da merkezi yönlendiricide uzak sayfalara kapatılır. Model anahtarı, geçmiş, ekler ve hafıza bu tarayıcıya verilmez. Kamera/mikrofon/konum/pano izinleri ve indirme kapalıdır. Web araştırmasını kapatmak bu görünümü kapatır.
- Üst düzey gezinme HTTPS/443 ve kimlik bilgisiz genel alan adlarıyla sınırlıdır; IP, yerel alan adı ve dosya/özel şema reddedilir. WebView2 olağan site betikleri, alt kaynaklar, yönlendirmeler, geçici çerezler ve sistem ağ yapılandırmasını kullanır. Önceki HTTP okuyucudaki DNS sabitlemesi, proxy kapatma ve 1 MB/sayfa sınırları bu tarayıcıya ait değildir. Eski okuyucu yalnız çekirdek/smoke örneğinde korunur. Bütün tarayıcı ağ trafiğinin yerel ağdan kesin izolasyonu iddia edilmez.
- En fazla 12 karar turu, 3 arama, ayara göre 1–5 kaynak, 180 saniye araştırma; 12.000 karakter sayfa metni ve kaynak başına 3.200 karakter alıntı uygulanır. Model yalnız gözlenmiş bağlantı/arama alanı indeksleriyle sınırlı JSON kararları verir. Genel JS, shell, hesap/ödeme/yükleme/mesaj işlemi yoktur. Tanınan aynı kökenli arama formları kullanılabilir; ayrıntılar [tarayıcı belgesindedir](RESEARCH_BROWSER.md). CAPTCHA çözülmez.
- Kaynak metinleri güvenilmeyen içerik olarak işaretlenir. Kaynak düğmeleri kayıtlı HTTPS kaynaklarını uygulama içindeki tarayıcıda açar. Okunamayan sayfa yerine arama özeti kaynak diye kaydedilmez. Kontrolü al üretimi iptal eder; okunan kaynaklar durdurulan turda da korunur.
- SQL girdileri parametrelidir. Tam yanıt ve kaynaklar atomik kaydedilir. Model/motor indirmeleri sabit sürüm ve SHA-256 kontrolünden geçer. Lisans dosyaları paketlenir.
- Dosya ekleri yalnızca Rust tarafındaki yerel dosya seçicisinden alınır; üretim komutuna UUID verilir. Belge/metin/görseller arama hizmetine gönderilmez, araştırma modunda ekler engellenir. Orijinal yollar ve EXIF saklanmaz; çıkarılmış metin ve JPEG'ler şifrelenmeyen SQLite/yedek verisidir. Konuşma silinince aynı mesaj satırındaki ekler de silinir.
- ZIP diske açılmaz; yol kaçışı, mutlak/sürücü yolları, sembolik bağlantı, şifreli kayıt ve 64 kayıt/64 MB açılmış boyut/200:1 oranı aşımı reddedilir. İç içe arşiv açılmaz. DOCX XML dış varlık/DTD tanımları kabul edilmez. PDF/Word betikleri ve ekli kod yürütülmez. Görsel boyutu/belleği sınırlandırılır, yeniden kodlanır. PDFium DLL'i hash doğrulamasıyla yalnızca paket yolundan yüklenir; sistem DLL araması yapılmaz.

## Açık sınırlar

SQLite şifreli değildir. Aynı Windows kullanıcısının süreçlerine veya yöneticiye karşı bellek, model anahtarı veya veritabanı gizliliği garanti edilmez. Dosya hash manifesti dijital imzanın yerine geçmez; dosya ile manifesti birlikte değiştirebilen bir saldırgan bu kontrolü geçebilir. Genel dağıtım için kod imzalama gerekir.

Model ayrı bir işletim sistemi güvenlik alanına alınmış değildir. Uygulama uzaktaki modele gitmez ve modele araç vermez; bunun bir OS ağ güvenlik duvarı veya sandbox olduğu iddia edilmez. Kullanıcı internetten indirilmiş model/motor dosyasını keyfî yolla seçemez.

İstem enjeksiyonuna karşı metin ayrımı kalite önlemidir; modelin anlamsal doğruluğunu garanti etmez. Kaynak numaraları ve kartlar kanıtı incelemeyi kolaylaştırır, her iddianın doğrulandığı anlamına gelmez. Tarayıcı yalnız okunmuş sayfa metnini kanıt olarak alır. Bağımsız eski HTTP smoke okuyucusu ise gövde alınamazsa arama özeti kullanıldığını belirtir. Alınan metinler kaynaklarla birlikte yerelde saklanır.

`secure_delete` ve WAL checkpoint normal SQLite silmeyi güçlendirir; SSD, dosya sistemi anlık görüntüleri, yedekler ve sistem geri yükleme kopyalarından adli silme garantisi sağlamaz. Kaldırıcı geçmişi korur. Geçmişi uygulama üzerinden silmek mümkündür.

Masaüstü varsayılanı Google'dır. Google sonuç arayüzü değişebilir veya doğrulama/olağan dışı trafik ekranı gösterebilir; bunlar kaynak sayılmaz ve otomatik geçilmez. Kullanıcı tarayıcıdan kontrolü alabilir. Bağımsız eski HTTP smoke okuyucusu DuckDuckGo kullanmaya devam eder; masaüstü araştırması bu okuyucuya düşmez. Hata durumunda uydurma kaynak üretilmez. Sözleşmeli bir arama API'si eklemek ayrıca gizlilik ve anahtar saklama tasarımı gerektirir.

## Yanıt devamı ve PDF yazma

Otomatik devam yalnızca aynı yerel motora gider; ek web araştırması başlatmaz. Yerel tokenizer ile bağlam denetlenir; 16 parça ve çıktı sınırı görünür `limited` durumudur. İptal devam parçalarını da durdurur.

PDF komutu kayıtlı konuşma/yanıt kimliği ve sınırlı formül SVG listesini kabul eder. SVG yolları eşleşen kayıtlı LaTeX formüllerine bağlanır; dış kaynak, DTD, olay ve izin dışı öğeler reddedilir. Hedef dosya Rust tarafındaki yerel kayıt penceresinde seçilir; WebView genel yol/yazma komutu gönderemez. Hash doğrulanan font ve PDFium kullanılır; metin HTML veya shell olarak çalıştırılmaz. Geçici dosya hedef klasörde benzersiz adla yazılıp atomik değiştirilir, hata halinde kaldırılır.

## Otomatik hafıza sınırları

Hafıza yerel SQLite'tadır ve şifreli değildir. Yeni kullanıcı mesajı ve önceden doğrulanmış kısa hafıza kayıtları, yalnız yerel çıkarım modeline gider. Mesaj/alıntı/kod temizleme ve hassas içerik filtreleri temkinlidir; kapsamlı DLP veya kusursuz anlamsal sınıflandırma iddiası yoktur. Sabit anahtarlar, literal kaynak/değer kontrolü, sınırlı JSON şeması ve boyut/adet sınırları uygulanır. Araştırmaya ham hafıza eklenmez; mevcut sorgu sözcük denetimi de korunur. Kullanıcının güncel araştırma sorusuna kendi yazdığı kişisel bilgiler önceki araştırma kuralları gereği aramaya girebilir.

Hafızasız sohbet hafıza okuma/yazmayı kapatır; sohbet geçmişini silen/gizleyen bir mod değildir. Sohbet silmek bağımsız hafıza kayıtlarını silmez. Unutma, ilgili alandaki değer/alıntı/kanıt/undo verisini siler ve anahtar blok kaydı bırakır. Fiziksel disk/WAL/yedekten adli silme garantisi verilmez. Bekleyen işlerin revizyon kontrolü, silinen veya düzeltilen bilgiyi eski bir model yanıtının yeniden yazmasını engeller. Windows hesabı başına mevcut uygulama veri dizini kullanılır; aynı Windows hesabını paylaşan kişiler için ayrı profil izolasyonu sunulmaz.
