# Aevra

**Kişisel zekâ, tam kontrol.** Windows için yerel sohbet, çeviri, matematik, kodlama ve kontrollü web araştırması uygulaması.

Tauri 2 + React + TypeScript · Rust · llama.cpp · Qwen3.5-9B Q4_K_M GGUF · SQLite.

## İndir ve kur

[Windows kurulum dosyaları](https://github.com/c0denail/aevra/releases/latest) Release sayfasındadır. `setup.exe`, aynı sürüme ait **beş `.bin` dosyası** ve `SHA256SUMS.txt` dosyasını aynı klasöre indirin; `setup.exe` dosyasını çalıştırın. Model ve çalıştırma motoru kuruluma dahildir. GitHub'ın otomatik oluşturduğu **Source code** arşivleri kurulum paketi değildir.

Kaynak kodlarını GitHub'a gönderme ve yeni kurulum sürümü yayımlama: [yayınlama rehberi](docs/GITHUB.md).

Bu depo gerçek modelle çalışan **0.1.0 prototipidir**. Son kullanıcı Ollama, Python, Node.js, Rust veya Cline kurmaz. Geliştirme araçları yalnızca uygulamayı derleyen bilgisayarda gerekir. Marka uygunluğu doğrulanmadı; Aevra bu projenin çalışma adıdır.

## Çalışan özellikler

- Yerel modelin uygulama tarafından başlatılması; yanıtların akış halinde gösterilmesi ve durdurulması.
- Ayarlardan CPU / GPU / Otomatik seçimi. Pakette ayrı CPU ve Vulkan motorları bulunur; Otomatik, uygun GPU kullanılamazsa CPU'ya döner. Donanım değişimi modeli yeniden hazırlar; açık GPU seçimi başarısızsa önceki tercih geri yüklenir.
- Matematik ve Kodlama çalışma alanları; yerel formül gösterimi ve dil etiketli kod blokları. Aynı Qwen modeli görev istemleriyle kullanılır; hesap makinesi veya kod yürütücüsü değildir. Sonuçlar ve üretilen kod ayrıca doğrulanmalıdır.
- Ataç düğmesiyle yerel PNG/JPG/WebP, PDF, Word (.docx/.doc), UTF-8/UTF-16 metin/kod ve ZIP ekleri. PDF metinleri çıkarılır; taranmış, formüllü ve görselli sayfalar görsel olarak da işlenir. Ekler kullanıcı mesajıyla SQLite'ta saklanır; durdurulan yanıtlarda da korunur.
- SQLite içinde kalıcı konuşmalar, çeviriler ve araştırma kaynakları; geçmişten açma ve konuşma silme.
- 11 hedef dil için metin çevirisi. Dil listesi bir kalite garantisi değildir.
- Araştırmada bağlamlı sohbet: yerel model yeni arama gerekip gerekmediğini belirler. Gerekirse güncel soru ve genel kaynak başlıklarıyla Google araması, ayarlanan 1–5 HTTPS kaynağı. Ham geçmiş dışarı gönderilmez; web kapalıyken önceki kaynaklarla sohbet sürer.
- Görünür araştırma tarayıcısı: konu yazıldığında Aevra sorguyu belirler, uygulamanın sağ panelindeki gerçek WebView2 tarayıcısında arama yapar, bağlantıları açıp okur ve kaynaklı yanıt hazırlar. Panel küçültülse de araştırma sürer. Adres çubuğu, geri/ileri, yenileme, işlem listesi ve Kontrolü al bulunur. Bir siteyi elle açıp “Bu sayfayı incele” veya “Bu sitede … ara” denebilir. Sesli konuşma bu özelliğe dahil değildir.
- Kategorili, kalıcı ayarlar: açık/koyu ana ekran, yazı boyutu, kişiselleştirme, yanıt tercihleri, arşivler, depolama ve yerel JSON yedekleri.
- Inter yazı tipi dahil çevrimdışı arayüz; beyaz/lavanta kartlar, ince ızgara, mor/turkuaz vurgular ve ae monogramı. Ana sayfadan çalışma alanlarına, son konuşmalara ve kaydedilmiş tercihlere erişilir; açık/koyu tema desteklenir.
- SHA-256 doğrulanan model/motor, uygulamaya dahil Microsoft C++ DLL'leri ve çevrimdışı WebView2 kurulumu.

## Bu bilgisayarda başlatma

```powershell
pnpm install --frozen-lockfile
pnpm assets:fetch
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\dev.ps1
```

Bu çalışma sırasında geliştirme için özel Rust ortamı `.tools/` altında hazırlanmıştır. `dev.ps1` onu otomatik bulur; global PATH değiştirilmez. Başka bir geliştirici bilgisayarında Node.js 22.12+ (veya 24+), pnpm 11, Rust stable ve Visual Studio C++ Build Tools/Windows SDK kurulu olmalıdır. Doğrulanan Rust sürümü: 1.99.1. Visual Studio yeniden dağıtım klasörü otomatik bulunamazsa `AEVRA_VC_REDIST_DIR` tanımlanabilir.

`pnpm dev` yalnızca tarayıcıda arayüz görünümünü açar; orada model ve SQLite erişimi yoktur, sahte yanıt üretilmez. Tam uygulama için `pnpm desktop` veya `dev.ps1` kullanın. Model uygulama açılır açılmaz arka planda doğrulanıp başlatılır. Durumu alt çubukta görünür; elle başlatma düğmesi yoktur.

Metin modeli 6.169.341.984 bayt; eşleşen görsel projector 918.165.952 bayttır. PDFium DLL'i de pakettedir. `assets:fetch` yalnızca geliştirici makinesinde model, projector, motor, PDFium ve WebView2 paketini indirir. Kurulu Aevra ilk açılışta model indirmez. İndirme adresleri, sürümler ve SHA-256 değerleri `distribution/assets.json` içinde sabittir.

## Doğrulama

Arayüz için `pnpm build` ve `pnpm test` çalıştırılır. İsteğe bağlı `pnpm test:ui`, üretim arayüzünü yerel önizleme sunucusunda ve ayrı headless Chrome profilinde 1240×820 / 820×620 boyutlarında denetler; açık/koyu ana sayfa, çeviri ve araştırma ekran görüntülerini `output/ui/` altına kaydeder. Geliştirme bilgisayarında Chrome gerekir; alternatif kurulu Chromium kanalı `AEVRA_TEST_BROWSER` ile seçilebilir. Bu yalnız geliştirici kontrolüdür; son kullanıcıya Chrome/Playwright/Node kurulmaz. Model ve kişisel sohbet verileri kullanılmaz.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\dev.ps1 -Task check
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\dev.ps1 -Task smoke
```

İlk komut arayüz derlemesini, arayüz güvenlik testlerini, Rust testlerini ve masaüstü derleme kontrolünü çalıştırır. İkincisi gerçek Qwen modelini başlatır, sohbet/çeviri üretir ve SQLite'ı yeniden açarak geçmişi doğrular. Web denemesi isteğe bağlıdır:

```powershell
cargo run -p aevra-core --example smoke -- --web
```

Bu son komut açık bir test sorgusunu internete gönderir. Normal testler ağ bağlantısı veya model gerektirmez.

Gerçek Windows WebView2 içinde site araması, 9B ile otomatik kaynak toplama, uzak sayfadan uygulama komutlarının engellenmesi ve iptal kontrolü için:

```powershell
cargo run --release -p aevra-desktop --features browser-smoke --example browser_smoke -- --web
```

Bu isteğe bağlı deneme interneti kullanır; ayrı `tmp/browser-smoke` test veritabanı ve raporu oluşturur, kişisel sohbet verilerine dokunmaz. Normal CI'a dahil değildir. Sitenin CAPTCHA istemesi testi durdurabilir. Kullanım ve kapsam: [araştırma tarayıcısı](docs/RESEARCH_BROWSER.md).

Açık Aevra oturumunun yanında ikinci bir model başlatmadan yalnız gerçek Google kutusu/sonuçları, kaynak okuma ve tarayıcı kontrollerini sınamak için aynı komuta `--browser-only` eklenebilir. Bu seçenek de açık `--web` gerektirir.

Yalnız CPU üzerinde gerçek 9B sohbet/çeviri/kayıt denemesi için `cargo run --release -p aevra-core --example smoke -- --cpu` kullanılabilir.

Uyumlu GPU ve gerçek paket dosyaları bulunan geliştirme bilgisayarında `cargo run --release -p aevra-core --example hardware`, GPU → CPU → Otomatik geçişini, matematik/kodlama üretimini ve SQLite kaydını sınar. Bu örnek açık GPU gerektirir; normal CI'da çalıştırılmaz. Üretilen kod yürütülmez; deneme kod doğruluğu garantisi vermez.

`cargo run --release -p aevra-core --example files` gerçek görsel projector/PDFium ile PNG, taranmış ve metinli PDF, DOCX, ZIP okumasını; GPU'da dört görseli, CPU'da görsel üretimini; kesilmiş geçmişi ve ekli yedek aktarımını sınar. İnternete bağlanmaz; normal CI'a dahil değildir.

`cargo run --release -p aevra-core --example repairs` Almanca/Türkçe çeviriyi, token sınırında otomatik devamı, 8.000 karakterden uzun PDF'nin tamamının çevrilmesini ve Türkçe/Almanca karakterli çok sayfalı PDF üretimini gerçek modelle sınar. Ağ kullanmaz; normal CI'a dahil değildir.

## Tam yanıt ve PDF oluşturma

Modelin `finish_reason=length` bildirdiği yanıt artık tamamlanmış sayılmaz; aynı yanıta otomatik devam edilir. Her akışın gerçek bitiş nedeni denetlenir. Bağlam 16.384 token'dır; yerel tokenizer ile metin sayılır, şablon/görsel payı ayrılır, eski bağlam gerektiğinde azaltılır. Kullanıcının seçtiği 256–8192 token bir üretim parçasının bütçesidir. 16 otomatik parça veya boyut sınırında yanıt `limited` olarak saklanır; tamamlandı diye gösterilmez. Sohbet/matematik/kodlamada Devam et düğmesi vardır. Beş dakikalık toplam istek zaman aşımı kaldırıldı; 180 saniyelik boş okuma sınırı ve kullanıcı iptali korunur. Bu sınırlar sınırsız üretim garantisi değildir.

Almanca hedefi modelde açıkça `German (Deutsch)` olarak kullanılır; diğer arayüz dil adları da sabit hedef adlarına eşlenir. Çeviri modunda ekler arama/ilgili bölüm seçicisine verilmez. Tüm metin bölümleri ve taranmış sayfalar sırayla çevrilir; dosya/sayfa etiketleri ve ilerleme gösterilir. Karakter sınırı nedeniyle alınamamış içerik varsa tam çeviri tamamlandı sayılmak yerine görünür hata verilir.

Yanıtın altındaki **PDF olarak kaydet** düğmesi yerel kayıt penceresi açar. Rust kayıtlı yanıt kimliği ve sınırlandırılmış formül SVG verilerini alır; genel dosya yazma/HTML/shell yetkisi UI'a verilmez. PDFium, pakette hash doğrulanan Noto Sans fontuyla A4 sayfalama, başlık, metin, kod ve kaynak listesi oluşturur. Türkçe/Almanca/Latin ve Kiril metinleri desteklenir. Desteklenmeyen tekil simgeler Unicode kodlarıyla belirtilir; desteklenmeyen yoğun yazı sistemleri açık hata verir. Formüller MathJax ile yerelde SVG yollarına dizilir; Rust resvg ile yüksek çözünürlükte işleyip PDF içine yerleştirir. Kesir, kök, integral, toplam ve matrisler formül olarak görünür. Gizli LaTeX katmanı formüllerin geri okunmasını sağlar. Kod içindeki dolar işaretleri korunur. Özgün PDF tasarımı yeniden oluşturulmaz. En fazla 200 formül, formül başına 10.000 karakter ve toplam 20 MB SVG kabul edilir. Kısmi yanıt PDF'de işaretlenir; kayıt geçici dosya + atomik değiştirme ile yapılır. Çıktı en fazla 4 MB metin / 500 sayfadır.

## Dosya okuma sınırları

- Bir mesajda en fazla 4 dosya, dosya başına 20 MB. Hazırlanan tüm eklerde en fazla 128 görsel/sayfa ve toplam 16 MB JPEG verisi. Görseller 1536×1536 sınırına küçültülüp JPEG'e dönüştürülür; özgün dosya ve EXIF verisi saklanmaz. Küçük yazılar/büyük tablolar için ilgili alanı kırpıp ekleyin.
- Bir ekte toplam en fazla 100 PDF sayfası. Metinli sayfalardan metin alınır; formül işaretleri/denklem veya gömülü görsel bulunan sayfalar ayrıca görüntü olarak alınır. Taranmış/az metinli sayfalar görsele çevrilir; çeviride tüm kabul edilmiş sayfalar işlenir. Sohbet/matematik/kodlamada ilk 4 görsel kullanılır. Şifreli PDF/ZIP desteklenmez. DOCX'te ana metin ve standart Office Math denklemleri alınır; Word 97–2003 .doc dosyalarında ana metin parça tablosundan okunur. Eski .doc içine gömülü denklem nesneleri desteklenmez; gömülü görseller, yorumlar ve makrolar işlenmez.
- Dosya başına çıkarılmış metin 1.000.000 karakterle sınırlıdır. Çeviride bütün bölümler doğal kelime/cümle sınırlarına yakın parçalarla sırayla işlenir. Diğer alanlarda ilgili bölümler seçilir; en fazla 8.000 karakter, görsel varsa 4.000 karakter aktarılır. Dosya ekli sorular 2.000 karakterle sınırlıdır; görsel başına motor üst sınırı 1536 token'dır. Sohbet modu tüm belgeyi görmüş sayılmaz; çeviride model doğruluğu/okunamayan görüntü kalitesi ayrıca değerlendirilmelidir.
- ZIP diske çıkarılmaz; en fazla 64 kayıt, toplam 64 MB açılmış boyut ve en fazla 200:1 sıkıştırma oranı. Tehlikeli yollar ve sembolik bağlantılar reddedilir. İç içe ZIP ve desteklenmeyen dosyalar atlanıp kartta belirtilir. Dosyalar/kodlar çalıştırılmaz.
- Araştırma alanına ek yüklenmez; hiçbir dosya içeriği arama sağlayıcısına gönderilmez. Yerel alanlarda yeni ek yoksa konuşmadaki en son ekler sonraki soru için yeniden kullanılabilir. JSON yedekleri ekleri içerir; eski biçim-1 yedekler alınır, yeni dışa aktarım biçim-2'dir. İçe aktarma 50 MB ile sınırlıdır.

## Otomatik kişisel hafıza

Varsayılan olarak açıktır; Ayarlar → Kişisel hafıza ekranından kapatılabilir. Yeni mesajlardan önemli bilgiler, yanıt tamamlandıktan sonra mevcut yerel 9B modeliyle çıkarılır. Model ağırlıkları değişmez. Açık kişisel bilgiler tek mesajdan; kısa/ayrıntılı/adım adım/örnekli yanıt gibi dolaylı tercihler aynı yönde üç farklı konuşmadan sonra kullanılır. İlgili en fazla altı kayıt yalnız son yerel yanıt istemine eklenir; araştırma planı veya sorgusu hafıza almaz.

Kayıtlar görüntülenir, dayanakları incelenir, düzenlenir ve unutulur. Otomatik güncelleme bildirimi on dakika boyunca Geri al sunar. Geri alma/unutma ilgili alanı otomatik öğrenmeye kapatır; ekrandan yalnız gelecekteki mesajlar için yeniden izin verilebilir. Eski sohbetler taranıp yeniden öğrenilmez. Yanıt kesilirse o turdan yeni çıkarım yapılmaz. Çeviri, ekli konuşmalar, alıntılar/kod ve belirgin hassas içerik otomatik çıkarıma verilmez. Bu temkinli filtre ve model sınıflandırması kusursuz bir hassas veri tespit sistemi değildir.

Üstteki Hafıza açık düğmesi sohbeti **Hafızasız sohbet** yapar; seçim o konuşma için kalıcıdır. Bu seçenek hafızayı okumaz/yazmaz; sohbet geçmişi kaydedilmeye devam eder. Geçici/silinmeden kalan sohbet geçmişiyle karıştırılmamalıdır. Hafıza bu Windows hesabının yerel veritabanına aittir; uygulama içinde ayrı kullanıcı profilleri yoktur. Sohbet silmek hafızayı silmez; sohbet JSON yedekleri hafızayı taşımaz. Veritabanı şifrelenmez.

İlk kapsam 13 alanla sınırlıdır: hitap adı, yanıt dili/uzunluğu, açıklama tercihi, meslek, öğrenme hedefi, Almanca/İngilizce seviyesi, programlama dili/teknolojileri, işletim sistemi, devam eden proje ve proje hedefi. Aynı alandaki bilgiler birleştirilir/güncellenir; tüm kişisel ayrıntıları hatırlayan sınırsız bir profil değildir. Geçici proje kayıtları 90, doğrulanmamış tercihler 30 gün sonra kullanılmaz. Yanıt sırasında arka plan işi iptal edilip kuyruğa bırakılır; sonraki boşlukta yeniden denenir. Düzenleme/silme/kapatma önceki kuyruk revizyonunu geçersiz kılar.

Gerçek model denemesi: cargo run --release -p aevra-core --example memory. İnternete çıkmaz; A2→B1 düzeltmesi, üç konuşmadan tercih, yeniden açılma ve yeni sohbetten hatırlama/unutma kontrol edilir. Normal CI bunu çalıştırmaz.

## Windows kurulumu üretme

```powershell
# Rust ortamının etkin olduğu terminalde:
node scripts/licenses.mjs
pnpm assets:fetch
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\dev.ps1 -Task bundle
```

Çıktılar:

- `artifacts/Aevra-0.1.0-windows-x64-offline.zip` — model, motor, kurulum EXE'si ve çevrimdışı WebView2 dahil dağıtım paketi.
- ZIP'i bir klasöre çıkarın; içindeki `Aevra-0.1.0-windows-x64-setup.exe` dosyasını açın. Yanındaki `.bin` dosyaları aynı klasörde kalmalıdır.
- ZIP'in `.sha256` dosyası ve içindeki `SHA256SUMS.txt` — bütünlük özetleri.
- `pnpm package:verify` — kaynak varlıklarını, uygulama klasörünü ve ZIP içindeki kurulum dosyalarını değiştirmeden doğrular; `target/` veya açık `artifacts/installer/` kopyası gerektirmez.
- `artifacts/Aevra-memory/Aevra.exe` ve `resources/` — kuruluma girecek uygulama klasörü; **birlikte** taşınmalıdır. Bu klasörü doğrudan çalıştırmak için hedef bilgisayarda WebView2 bulunmalıdır.

6,17 GB model ve 0,92 GB görsel bileşen nedeniyle kurulum verisi Inno Setup `.bin` dilimlerine ayrılır; EXE ve dilimler tek ZIP64 dağıtım paketinde taşınır. Kurulum internetten model indirmez. `distribution/Aevra.iss` kullanıcıya özel kurulum ve kaldırma işlemini tanımlar. Derleyici geliştirme bağımlılığı olarak bulunur; hedef bilgisayara kurulmaz. İstenirse `AEVRA_ISCC` ile başka bir Inno Setup derleyicisi seçilebilir. Tek bağımsız kurulum EXE'si henüz bu dağıtım profilinin çıktısı değildir.

Açık uygulama eski dosyayı kullanıyorsa `AEVRA_PAYLOAD_DIR` ile yeni uygulama klasörü seçilebilir; derleyici aynı klasörü kuruluma alır. Bu geliştirme turunun güncel uygulama klasörü `artifacts/Aevra-memory/` içindedir. Eski Aevra'yı kapattıktan sonra yeni uygulamayı açın; önceki klasörler değiştirilmez.

`target/`, `dist/`, `tmp/`, `output/` ve `src-tauri/gen/` yeniden üretilebilen derleme/QA çıktılarıdır. Rust derleme çıktıları `cargo clean`, TypeScript önbelleği `pnpm exec tsc --build --clean` ile temizlenebilir. Geliştirme için kullanılan `.tools/cargo`, `.tools/rustup` ve `node_modules` korunur. Güncel uygulama klasörü ile çevrimdışı ZIP, dağıtım çıktılarıdır.

Kurulum Windows 10 22H2 / Windows 11 x64 içindir. Varsayılan donanım seçimi Otomatik'tir. GPU için uyumlu Vulkan ekran kartı sürücüsü gerekir; yoksa CPU kullanılabilir. En az 16 GB RAM, görsel/büyük belge kullanımı için 24–32 GB RAM önerilir; GPU bellek gereksinimi ve hız donanıma göre ayrıca ölçülmelidir. Bu bilgisayarda RTX 3070 Ti Laptop 8 GB ile GPU üretimi doğrulandı. Uygulama ve kurulum henüz kod imzalı değildir. Temiz Windows sanal makinesinde çevrimdışı kurulum kabul testi ve imzalama, genel dağıtım öncesi yapılmalıdır.

## Gizlilik ve kapsam

Geçmiş ve tercihler `%LOCALAPPDATA%\com.aevra.desktop\aevra.sqlite3` konumundadır. Uygulama kaldırıldığında kullanıcı verisi otomatik silinmez. Veritabanı bu sürümde şifrelenmez; ek metinleri ve görseller de aynı veritabanındadır. Kullanıcı mesajı üretimden önce kaydedilir; durdurulan veya hata alan yanıtın yazılmış bölümü de durumu ve kaynaklarıyla saklanır. Gönderilen metin yazı alanından hemen temizlenir; yanıt sırasında yeni taslak yazılabilir. v1/v2/v3/v4 geçmişi, arşivler ve kesilmiş yanıtlar korunarak v5'e göç ettirilir.

Sohbet, çeviri, matematik ve kodlama, yerel motor dışında ağ isteği yapmaz. Formül yazı tipleri de pakettedir. Araştırma modundan gönderilen soru ek onay penceresi olmadan uygulama içindeki tarayıcıyla araştırılır. Panel küçültülürse araştırma sürer; araştırma modundan çıkınca uzak tarayıcı görünümü kapatılır. Araştırma ayarlardan kapatılabilir. IP ve bağlantı üstverileri hizmetlerce görülebilir. Kaynak düğmesi bağlantıyı uygulamanın içinde açar. WebView2'nin kendi Microsoft güncelleme davranışı uygulamadan ayrıdır.

Web araması resmi API/SLA içermeyen bir önizleme bağdaştırıcısıdır; hizmet kısıtlamasında görünür hata döner. Sayfa alınamazsa arama özeti olduğu açıkça belirtilir. Kaynak sunulması, modelin her iddiasının doğru olduğunu kanıtlamaz.

Şifreli geçmiş, kod çalıştırma, tam metin arama ve otomatik güncelleme bu sürüme dahil değildir. Ayrıntılar: [mimari](docs/ARCHITECTURE.md), [güvenlik](docs/SECURITY.md), [geliştirme sırası](docs/ROADMAP.md), [kabul testleri](docs/ACCEPTANCE.md).

## Teknik kaynaklar

- [Tauri kaynak dosyası dağıtımı](https://v2.tauri.app/develop/resources/) ve [Windows dağıtımı](https://v2.tauri.app/distribute/windows-installer/).
- [llama.cpp sunucusu](https://github.com/ggml-org/llama.cpp/blob/b9222/tools/server/README.md).
- [Qwen3.5-9B model kartı](https://huggingface.co/Qwen/Qwen3.5-9B) ve [kullanılan GGUF türevi](https://huggingface.co/bartowski/Qwen_Qwen3.5-9B-GGUF).
- [NSIS büyük kurulum sınırı](https://nsis.sourceforge.io/I_get_an_error_when_compiling_large_installers), [Inno Setup tek dosya sınırı](https://jrsoftware.org/ishelp/topic_setup_diskspanning.htm).
- [WebView2 dağıtım seçenekleri](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution).

## 9B ve formül doğrulaması

`node scripts/prepare-math-sample.mjs` arayüzün gerçek MathJax dönüştürücüsüyle örnek hazırlar. `cargo run --release -p aevra-core --example upgrade -- --web` formüllü iki sayfalık PDF üretir, yeniden okur, 9B ile görsel denklem çözümü ve bağlamlı araştırma planını sınar. `--web` gerçek internet isteği yapar; normal CI bu örneği çalıştırmaz. Yalnız PDF kontrolü için `--pdf-only` kullanın.
