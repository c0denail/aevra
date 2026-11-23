# GitHub ve Release yayımlama

Proje: https://github.com/c0denail/aevra

Kaynak kodu Git deposunda; Windows kurulum dosyaları **Releases** bölümünde tutulur. `.gitignore`, model/motor dosyalarını, kurulumları, geliştirme araçlarını, önbellekleri ve SQLite sohbet verilerini dışarıda bırakır. Bunları `git add -f` ile eklemeyin.

## İlk kez başka bir bilgisayardan bağlanma

[Git](https://git-scm.com/downloads/win) ve [GitHub CLI](https://cli.github.com/) kurulduktan sonra PowerShell'de:

```powershell
gh auth login --hostname github.com --git-protocol https --web
gh auth setup-git
git clone https://github.com/c0denail/aevra.git
Set-Location aevra
```

Tarayıcıdaki GitHub oturumunu kendiniz tamamlayın; şifre veya erişim anahtarını sohbete yapıştırmayın. Kod değişiklikleri için `git add`, `git commit` ve `git push` kullanılır. Uygulamayı derleme gereksinimleri ana README'dedir; yalnız kurulum dosyasını indiren kullanıcıya bu araçlar gerekmez.

## İlk depoyu oluşturma

Bu adımlar yalnız henüz uzak deposu ve commit'i olmayan yerel proje içindir. Git kullanıcı adı/e-postası ayarlı olmalıdır; herkese açık commit'lerde GitHub hesabınızın gizli e-posta adresi kullanılabilir.

```powershell
git add .
git commit -m "Initial Aevra application"
git branch -M main
gh repo create aevra --public --source . --remote origin --push
```

GitHub web sitesinden oluşturacaksanız **New repository → aevra → Public** seçin. Yerel projede zaten bulunduğu için README, lisans veya `.gitignore` oluşturma seçeneklerini işaretlemeyin. Sonra verilen depo adresini `git remote add origin ADRES` ile ekleyip `git push -u origin main` çalıştırın. Bu proje yayımlandıktan sonra bu ilk kurulum adımlarını tekrarlamayın.

## Kurulumu Release'e ekleme

GitHub, Release dosyalarının her birinin **2 GiB'den küçük** olmasını ister. Yaklaşık 7,35 GB'lık tek çevrimdışı ZIP bu sınıra uymaz. Mevcut Inno Setup kurulumu zaten uygun boyutlarda parçalanmıştır; yeniden bölmek veya Git LFS kullanmak gerekmez.

0.1.0 sürümü için `artifacts/installer/` içindeki şu **yedi** dosya yüklenir:

- `Aevra-0.1.0-windows-x64-setup.exe`
- `Aevra-0.1.0-windows-x64-setup-1.bin`
- `Aevra-0.1.0-windows-x64-setup-2.bin`
- `Aevra-0.1.0-windows-x64-setup-3.bin`
- `Aevra-0.1.0-windows-x64-setup-4.bin`
- `Aevra-0.1.0-windows-x64-setup-5.bin`
- `SHA256SUMS.txt`

Önce dosyaları doğrulayın, sonra Release taslağına yükleyin:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\publish-release.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\publish-release.ps1 -Repository c0denail/aevra -Upload
```

İlk komut yalnız okur; dosyaların SHA-256 özetlerini, adlarını ve boyutlarını denetler. İkincisi mevcut commit'i hedefleyen `v0.1.0` Release taslağı oluşturur ve dosyaları yükler. Bağlantı kesilirse aynı komut tekrar çalıştırılabilir; GitHub'daki özeti eşleşen dosyalar atlanır. Yayımlanmış sürümün üzerine yazılmaz.

GitHub'da **Releases → Drafts** bölümünden açıklamayı ve yedi dosyayı inceleyin; **Publish release** ile yayımlayın. Alternatif olarak:

```powershell
gh release edit v0.1.0 --repo c0denail/aevra --draft=false --latest
```

Elle yüklemede **Releases → Draft a new release** üzerinden `v0.1.0` etiketini ve ilgili commit'i seçin, `docs/releases/v0.1.0.md` metnini açıklamaya kopyalayın ve yedi dosyayı ekleyin. Büyük dosyalarda yüklemenin tamamlanmasını bekleyin.

## Sonraki sürümler

Uygulama sürümünü `package.json`, `Cargo.toml`, Tauri ayarları ve dağıtım betiklerinde birlikte yükseltin. Kurulumu yeniden üretip `pnpm package:verify` ile denetleyin, ilgili `docs/releases/vX.Y.Z.md` açıklamasını yazın, kaynakları commit edip gönderin; sonra yayınlama betiğini çalıştırın. Eski sürümün kurulum dosyalarını yeni etiket altında yayımlamayın.

Resmî kaynaklar: [yerel projeyi GitHub'a ekleme](https://docs.github.com/en/migrations/importing-source-code/using-the-command-line-to-import-source-code/adding-locally-hosted-code-to-github?platform=windows), [Release dosya sınırları](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases), [GitHub CLI Release oluşturma](https://cli.github.com/manual/gh_release_create).
