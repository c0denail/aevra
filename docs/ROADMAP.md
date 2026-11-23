# Geliştirme sırası

1. **Temel ve mimari — hazır:** Tauri/React iskeleti, ayrı Rust çekirdeği, Türkçe arayüz ve logo.
2. **Yerel model — hazır:** paket doğrulama, alt süreç, sağlık kontrolü, SSE akışı, iptal, CPU/Vulkan motorları ve CPU/GPU/Otomatik ayarı.
3. **Kalıcı geçmiş — hazır:** SQLite v4, konuşma açma/silme, üretimden önce eklerle kullanıcı mesajı, durumuyla tam/kısmi yanıt ve kaynak saklama.
4. **Çeviri — hazır:** metin, izinli hedef dil listesi ve yerel işlem.
5. **Web araştırması — prototip hazır:** doğrudan arama, kapatma tercihi, kaynak sayısı ayarı, kaynak çıkarımı, DNS/URL denetimi. Sağlayıcı dayanıklılığı ve kaynak/iddia kalitesi sonraki iyileştirmelerdir.
6. **Dağıtım — ilk paket üretildi:** model + C++ motoru + DLL + offline WebView2 içeren kurulum EXE'si ve veri dilimleri tek ZIP64 paketinde. Tek bağımsız EXE profili, imzalama ve temiz Windows kabul testi sonraki işlerdir.
7. **Matematik ve kodlama — hazır:** görev istemleri, çevrimdışı formül gösterimi, kod blokları ve kalıcı geçmiş. Gerçek RTX 3070 Ti GPU/CPU geçişleri denendi; model kalitesi ve üretilen kodun doğruluğu ayrı değerlendirilmelidir.
8. **Sonraki dilim:** farklı donanımlarda süre/bellek ölçümleri, Vulkan sürücüsüz sistemde otomatik CPU dönüşü, kurulum öncesi donanım denetimi ve temiz Windows kabulü.
9. **Dosya okuma — ilk kapsam hazır:** yerel PNG/JPG/WebP, metin/kod, DOCX ana metni, metinli/taranmış PDF ve güvenli ZIP; ekli geçmiş/yedekler. Gerçek CPU/GPU görsel ve PDF/DOCX/ZIP model denemeleri geçti. Sayfa seçimi, genel PDF vektör/görsel seçimi, büyük taranmış belgeler için özel OCR ve daha güçlü belge araması sonraki işlerdir.
10. **Tam yanıt / tam belge çevirisi / PDF çıktısı — hazır:** bitiş nedeni denetimi ve otomatik devam; açık Almanca hedefi; belge bölümlerinin hepsini sırayla çevirme; yerel, sayfalı metin PDF çıktısı ve Noto Sans. Gerçek model ve bağımsız PDF okuyucu ile örnekler doğrulandı.
11. **Ardından:** Windows DPAPI ile anahtar korunan şifreli geçmiş, FTS araması, model değiştirme ve imzalı güncellemeler. Yerel JSON yedekleri ve arşiv yönetimi uygulanmıştır.

İlk sürümün kapsamını koruyun. Bir özellik gerçekten bağlanmadan arayüzde çalışıyormuş gibi sunulmasın. Her dilim için önce kabul davranışı, sonra uygulama, sonra ilgili doğrulama yapılır.

12. **9B / araştırma sohbeti / formüllü belgeler:** Qwen3.5-9B ve eşleşen projector gerçek GPU/CPU denemeleriyle doğrulandı. Araştırma yerel planlayıcı ve önceki kaynaklarla konuşur; MathJax/resvg PDF formüllerini diziler; formüllü PDF görselleri modele verilir; DOCX OMML ve eski DOC metni okunur. Genel kalite, büyük belgeler ve temiz Windows kabulü ayrı kapılardır.
13. **Otomatik kişisel hafıza — ilk kapsam hazır:** kaynakla doğrulanan kişisel bilgiler ve tekrarlanan üslup gözlemleri, SQLite v5 kayıt/kanıt/kuyruk/unutma blokları, ilgili yerel bağlam, hafızasız sohbet, yönetim ve geri alma. 9B ile A2→B1 ve yeni sohbetten hatırlama doğrulandı. Ayrı uygulama profilleri, şifreleme ve hafıza dışa aktarma sonraki işlerdir.
