# CLAUDE.md

Hướng dẫn cho Claude Code (claude.ai/code) khi làm việc trong repo này.

## Ngôn ngữ

Toàn bộ mã nguồn, comment, tên biến/hàm, tên file và commit message dùng **tiếng
Việt** (không dấu cho định danh, có dấu trong comment). Giữ nguyên quy ước này —
đừng chen tên tiếng Anh vào giữa.

Comment ở đầu mỗi file giải thích *vì sao* chứ không phải *cái gì*, và thường ghi
kèm **số đo thực nghiệm**. Sửa logic có số đo thì cập nhật luôn con số. Gần như
mọi lưới chặn trong repo này đều suy từ số liệu đo trên sách thật chứ không từ
suy đoán — comment phải nói ra số liệu ấy, không thì người sau tưởng là tuỳ tiện
và bỏ đi.

## Lệnh thường dùng

```bash
cargo build --release
cargo test --release --workspace
cargo run --release -p giaodien --bin vie-spellcheck      # cửa sổ
cargo run --release -p giaodien --bin vsc -- sach.epub --kho   # dòng lệnh, chạy khô
```

**Dùng `--release` cho cả `cargo test`.** Không có cờ ấy thì cargo dựng
llama-cpp-sys-2 ở profile debug, tức là **biên dịch lại toàn bộ nhân CUDA của
llama.cpp** — hàng chục phút, để chạy một bộ kiểm mất hai giây.

### Dựng cần gì

- **Visual Studio Build Tools** (workload C++), **cmake**, và **`LIBCLANG_PATH`**
  trỏ vào thư mục chứa `libclang.dll` — `llama-cpp-sys-2` dựng llama.cpp bằng
  cmake và sinh binding bằng bindgen.
- **CUDA Toolkit ≥ 12.8** (card Blackwell / RTX 50xx cần từ bản này).

Dựng không cần CUDA: `cargo build --release --no-default-features`. Bản ấy chạy
được mọi tầng luật, chỉ không nạp được mô hình.

### Ba cạm bẫy của bản dựng CUDA

**`MSB1009: Project file does not exist`** nghĩa là cmake đã hỏng dở một lần
trước đó và để lại `CMakeCache.txt`; lần sau nó bỏ qua bước cấu hình rồi MSBuild
không có gì để dựng. `cargo clean` **không** gỡ được — phải xoá hẳn:

```bash
rm -rf target/release/build/llama-cpp-sys-2-* target/debug/build/llama-cpp-sys-2-*
```

**Thiếu `CUDA_PATH_V13_3`** thì cmake báo `The CUDA Toolkit directory '' does not
exist`. Bản cài đặt CUDA có đặt biến ấy ở mức máy, nhưng **shell mở trước lúc cài
thì không thấy nó** — mở terminal mới.

**`cuda-arch.cmake` ghim kiến trúc CUDA cần dựng** (`120a-real` = Blackwell,
RTX 50xx). Đổi card thì sửa file ấy. Không có nó thì llama.cpp dựng cho cả bảy
đời card và mất hàng chục phút để sinh ra sáu bản mã máy này không bao giờ chạy.
`.cargo/config.toml` trỏ cmake vào file ấy qua `CMAKE_TOOLCHAIN_FILE`; dùng
đường vòng ấy vì `build.rs` của llama-cpp-sys-2 tự đặt `GGML_NATIVE=OFF` sau khi
đọc biến môi trường nên đặt biến không ăn thua.

**CUDA 13 dời DLL runtime sang `bin\x64`.** `crates/giaodien/build.rs` chép
`cublas64_*.dll`, `cublasLt64_*.dll`, `cudart64_*.dll` sang cạnh file thực thi.
Thiếu chúng thì Windows chặn ngay ở bước nạp ảnh — hộp thoại đỏ "The code
execution cannot proceed", chưa vào được dòng `main` nào nên không có chỗ nào báo
lỗi tử tế hơn. Đây không phải chuyện chỉ thiếu PATH: cho `%CUDA_PATH%\bin` vào
PATH cũng không đủ vì DLL không còn nằm ở đó nữa.

## Kiến trúc

```
crates/chinhta/    lõi ngôn ngữ — không phụ thuộc EPUB hay llama.cpp
  am_tiet.rs       cấu tạo âm tiết: âm đầu + vần + thanh
  tu_dien.rs       kho âm tiết và kho từ ghép; tách chữ dính
  dau_thanh.rs     vị trí dấu thanh, kiểu cũ/mới
  chuan_hoa.rs     Unicode, khoảng trắng, dấu câu
  de_nham.rs       cặp dễ nhầm (bảng ở du-lieu/de-nham.txt)
  ung_vien.rs      từ tiếng sai sinh ra các cách sửa
  doi_chieu.rs     so bản trước/sau, ra khoảng byte cần vá
  soat.rs          điều phối các tầng
crates/sach/       đọc/ghi EPUB giữ nguyên phần không sửa
crates/mohinh/     chấm điểm câu bằng llama.cpp trên GPU
crates/giaodien/   ứng dụng (cửa sổ egui + dòng lệnh), nhật ký, báo cáo
```

### Đường đi của dữ liệu

```
EPUB → sach::quet     tách file XHTML thành đoạn, nối các nút bị thẻ inline cắt
     → chinhta::soat  chạy các tầng, ra (chữ đã sửa, danh sách chỗ ngờ)
     → mohinh         chấm điểm chọn giữa các ứng viên  [tuỳ chọn]
     → doi_chieu      so bản trước/sau → khoảng byte cần vá
     → sach::ghi      vá đúng khoảng ấy, chép nguyên phần còn lại
```

## Những quyết định định hình cả phần còn lại

### Ứng dụng tự sửa rồi mới báo cáo

Nên **sửa nhầm đắt hơn bỏ sót nhiều lần**. Mọi lưới chặn trong repo đều nghiêng
về phía ấy. Khi cân nhắc thêm một luật mới, hỏi: luật này làm hỏng chữ đúng
trong bao nhiêu phần trăm ca? Nếu không đo được thì đừng thêm.

Bản gốc **không bao giờ bị ghi đè** — kết quả luôn ra file khác, và `bat_dau`
chặn cả khi người dùng cố chọn trùng.

### Mô hình ngôn ngữ **không được sinh ra chữ nào**

Nó chỉ chấm điểm câu. Các cách sửa do tầng luật và tầng cấu tạo âm tiết sinh ra —
chúng bảo đảm mọi ứng viên đều là tiếng Việt viết đúng — còn mô hình chỉ **xếp
hạng**. Nhờ thế nó không viết lại được văn của tác giả, không bịa tên riêng, và
kết quả lặp lại được nên kiểm thử được.

Đừng đổi sang kiểu "đưa câu cho mô hình bảo nó sửa". Đã cân nhắc và bác: mô hình
tiện tay đổi từ ngữ, gộp câu, bỏ chữ nó thấy thừa. Trong tiểu thuyết thì đó
không phải sửa lỗi, đó là hỏng sách.

### Ba tầng xác nhận một tiếng, thứ tự không đảo được

1. **Từ điển** (9.550 âm tiết) — có thì thôi.
2. **Cấu tạo âm tiết** — đỡ cho những gì từ điển thiếu.
3. Trượt cả hai mới sinh cách sửa, rồi **từ ghép** (69.893 mục) chọn giữa chúng.

Chỉ dùng bảng vần thì **1.813 từ mượn** viết theo âm Việt (`bêtông`, `micrô`,
`rađa`, `pittông`) bị sửa hỏng — bảng vần mô hình hoá ngữ âm tiếng Việt nên nó
bác bỏ mọi thứ ngoài hệ thống ấy. Chỉ dùng từ điển thì mọi chữ lạ hợp lệ đều bị
bắt, và `híc` không nằm trong bộ từ điển nào cả.

### Thứ tự chọn ứng viên, và mô hình đứng **cuối**

Xếp hạng theo năm tiêu chí, thứ tự không đảo được:

1. **Bảng typo hay gặp** — `du-lieu/typo.txt`, xem `chinhta::typo`.
2. **Cụm tên riêng của cuốn sách** — hai tiếng viết hoa liền nhau giữa câu, lặp từ
   ba lần.
3. **Giữ được phụ âm đầu.**
4. **Từ ghép trong từ điển phổ thông** (69.893 mục).
5. **Giá của phép sửa** — xem `ung_vien`.

Rồi bằng chứng nào đủ mạnh thì áp thẳng, không hỏi mô hình: `Typo` (bảng chỉ ghi
đúng một đáp án), `CumTenRieng`, `TuGhep`, hoặc `DauPhu` (chỉ thêm đúng một dấu
phụ, và chữ ấy có trong vốn từ của cuốn sách). Còn lại mới tới **mô hình ngôn ngữ**.

Đo trên Phàm Nhân Tu Tiên, ai quyết bao nhiêu chỗ: typo 484, từ ghép 491, tách chữ
dính 175, dấu phụ 63, tên riêng 35, mô hình 365.

**Cụm `uơ` có phụ âm cuối đi chung hạng với bảng typo**, vì nó cũng là quan sát
trực tiếp — chỉ khác chỗ suy ra được thay vì phải tra bảng. Tiếng Việt viết `ươ`,
và bảng 9.550 âm tiết không có một chữ nào phản bác: sáu chữ mang cụm `uơ` thật
(`huơ`, `khuơ`, `khuờ`, `nguơ`, `thuở`, `uở`) đều là **vần mở**, còn ba chữ có phụ
âm cuối (`quờn`, `quởn`, `quớt`) đều thuộc nhóm `qu-`, nơi `u` là âm đầu chứ không
phải cụm nguyên âm.

**Dấu thanh giữ nguyên**, và chính điều ấy làm phép sửa này chỉ có *một* đáp án:
`cuờng` ra `cường` chứ không phải `cưỡng`. Đo trên sách thì cả hai chỗ đổi đều sửa
được lỗi — `thần thông cưỡng đại` thành `cường đại`, `vườn một ngón tay` thành
`vươn một ngón tay`.

Thiếu vế "có phụ âm cuối" thì luật quét cả `thuở`, mà `muôn thuở`, `thuở nhỏ` đầy
trong sách còn `thưở` lại đúng là lỗi rất nhiều người viết — bộ sửa sẽ **tạo ra**
đúng lỗi ấy ở những chỗ tác giả viết đúng.

**Bảng typo đứng trên hết vì nó là quan sát trực tiếp.** `khôgn` gặp 153 lần trong
một bộ truyện và lần nào cũng là `không`. Mọi tầng dưới đều đang **suy** — suy từ
bàn phím (giá của phép sửa), từ từ điển (từ ghép), từ thống kê của cuốn sách (cụm
tên riêng).

Nhưng **phải giữ cả mục nhiều đáp án**, và đây là chỗ dễ làm hỏng nhất. `măt` gặp
20 lần trong hai bộ truyện: 12 lần đúng là `mắt`, 7 lần là `mặt`, 1 lần là `mật`.
Bảng nào chỉ giữ đáp án đông nhất sẽ sai 8 trong 20 chỗ ấy, và sai **im lặng** vì
nó xếp trên mọi tầng khác. Nên mục nhiều đáp án không quyết gì cả; nó chỉ **thu
hẹp** danh sách ứng viên xuống đúng những chữ đã thật sự gặp. Trong 97 mục của bảng
thì 72 mục một đáp án và 25 mục nhiều đáp án — tức một phần tư số typo hay gặp
không tự quyết được.

**Giữ phụ âm đầu phải đứng trên từ điển phổ thông.** Đo trên Phàm Nhân Tu Tiên khi
gỡ nó ra khỏi đường xếp hạng: phép thay phụ âm đầu 29 → 42, thời gian **457 giây →
1.537 giây** (tầng luật loại được ít ứng viên hơn nên mô hình phải chấm gấp rưỡi),
và mấy ca đã vá xong quay lại — `duợc` ra `cuộc` (vì `cuộc gọi` có trong
`tu-ghep.txt`), `mẹnh` ra `lệnh` còn `lẹnh` ra `mệnh`.

**Bảng typo không thay được luật ấy, và ngược lại.** Bật riêng từng cái trên Phàm
Nhân Tu Tiên:

| | chỉ luật phụ âm | chỉ bảng typo | **cả hai** |
|---|---|---|---|
| Lỗi chữ nghĩa | 2.371 | 2.379 | 2.377 |
| Phép thay phụ âm đầu | 27 | 42 | **29** |
| Thời gian | 258 giây | 1.537 giây | 457 giây |

**Cụm tên riêng: chỉ gom tên riêng, đừng gom cụm thường.** Từng thử một bảng chứa
mọi cặp tiếng hay gặp (ngưỡng 8, ra 63.964 cụm) và một bảng ba hạng (tên riêng /
cụm ba tiếng ngưỡng 10 / cụm hai tiếng ngưỡng 100). Cả hai đều tệ hơn, vì cụm
thường là thống kê chứ không phải từ điển do người biên soạn: đặt trên từ ghép thì
`chúg tôi` ra `chủ`, `biết điề` ra `đi`, `bỏ trốngm` ra `trong`, `Thấy ẩnh` ra
`anh`, `câi chuyện` ra `cái`. Nâng ngưỡng không chữa được — trong bộ 25 triệu chữ
thì cặp thường cũng lặp hàng nghìn lần, mà `hỏa dương` chỉ gặp 39.

Hai cái van của bảng tên riêng, vì nó đứng trên gần hết mọi tầng:

- **Chữ đáng ngờ thì không được vào.** Cả hai tiếng phải **có trong từ điển âm
  tiết**, không chỉ hợp cấu tạo. Cái giá là mất mấy tên phiên âm ngoài từ điển
  (`Kông` trong `Hồng Kông`), nhưng chúng đã có `gom_ten_rieng` che theo tiêu chí
  khác.
- **Cụm đã có trong `tu-ghep.txt` thì không lấy.** Bảng này để chứa cái từ điển
  phổ thông *không* biết. Chép lại `chúng ta`, `cuộc gọi`, `cao tầng` vào đây thì
  vừa thừa — tầng từ ghép vẫn lo chúng — vừa nguy, vì nó nhấc một từ ghép tầm
  thường lên trên luật giữ phụ âm đầu.

**Bỏ tiếng mở đầu câu.** Đầu câu thì chữ nào cũng viết hoa nên chữ hoa ở đó chẳng
nói gì; không có luật này thì `Nhưng Hàn Lập` thành một cụm tên riêng lặp hàng trăm
lần.

**Chỉ tra bảng cho chữ viết hoa.** Bảng dựng từ cặp *viết hoa* — đó là toàn bộ tín
hiệu của nó — nên đem tra cho chữ thường là dùng bằng chứng ở chỗ nó không còn giá
trị. Đo được: `Mệnh Bài` viết hoa 14 lần nên vào bảng, `Lệnh Bài` chỉ 1 lần nên
trượt ngưỡng; nhưng tính cả chữ thường thì `lệnh bài` gặp **463** lần còn `mệnh
bài` chỉ 42. Thế là `phía trên lẹnh bài` ra `mệnh bài`, dù `lệnh` vừa giữ phụ âm
đầu vừa rẻ hơn bốn lần. Thêm van này thì số chỗ bảng tên riêng quyết đi từ 97 xuống
35 — hai phần ba số ấy là chữ thường, nơi nó không nên có tiếng nói.

**Bảng đếm phải hai tầng.** Bộ truyện 25 triệu chữ có chừng sáu triệu cặp tiếng
liền nhau, phần lớn xuất hiện đúng một lần. Giữ nguyên chữ cho tất cả thì bảng ngốn
hàng trăm MB. Nên cặp gặp lần đầu chỉ để lại dấu vân tay 64 bit, sang lần thứ hai
mới giữ chữ.

### Vốn từ của cuốn sách lấp chỗ `am-tiet.txt` không lấp được

`am-tiet.txt` là bảng **âm tiết hợp cấu tạo**, nên nó chứa cả những âm tiết chẳng
ai viết. `đêu` nằm trong bảng. Vì thế `hắn chỉ đeu găng tay` bị sửa thành `đêu găng
tay`: ứng viên "có trong từ điển", chỉ thêm một dấu phụ, rẻ nhất bảng — mà vẫn sai,
và từ đúng là `đeo`.

`gom_tu_dung` đếm mọi tiếng cuốn sách thật sự dùng (Harry Potter: 2.910 tiếng khác
nhau; Phàm Nhân Tu Tiên: 4.263), ngưỡng ba lần. Luật `DauPhu` đòi ứng viên phải có
trong bảng ấy. `đeo` có, `đêu` không.

### Hai lối hỏi mô hình

Mặc định là lối hiển nhiên: thay từng ứng viên vào rồi **chấm cả câu**, so xem
câu nào tự nhiên hơn (`KieuCham::CaCau`, ngưỡng 0,03).

Lối kia có sẵn sau cờ `vsc --cho-trong`: khoét chữ sai thành **chỗ trống**, đưa mô
hình hai câu trước và hai câu sau, rồi chỉ chấm **phần điền vào cùng phần đuôi**,
còn phần đứng trước chỉ để đọc chứ không tính điểm (`KieuCham::ChoTrong`, ngưỡng
0,018). Mục này ghi lại số đo của cả hai để lần sau không phải đo lại.

Đếm tổng thì hai lối trông như nhau — 126 so với 125 lỗi chữ nghĩa trên Harry
Potter tập 4, 2.296 so với 2.276 trên Phàm Nhân Tu Tiên (25,6 triệu chữ), đều để
lại 0 chỗ ngờ. Phải đếm theo **loại sai** mới thấy khác. Trên 8 chỗ hai lối quyết
khác nhau ở Harry Potter, mỗi lối ở ngưỡng đo riêng cho nó:

| | Sửa nhầm | Bỏ sót | Thời gian |
|---|---|---|---|
| Cả câu, ngưỡng 0,03 | 5 | 1 | 69 giây |
| Chỗ trống, ngưỡng 0,018 | **3** | **0** | 114 giây |

Phàm Nhân Tu Tiên thì 64 chỗ quyết khác nhau, nhiều quá để đọc tay hết, nên đo
bằng thước đúng là rủi ro của truyện tu tiên — phép sửa có **đổi phụ âm đầu**,
tức có biến tên người này thành tên người khác, hay không:

| | Đổi phụ âm đầu | Trong đó là chữ hoa (gần như chắc là tên riêng) |
|---|---|---|
| Cả câu | 30 / 601 phép sửa | 4 (`Duơng` → `Hương`, `Môc` → `Lộc`, `Măc` → `Hắc`, `Bhận` → `Chân`) |
| Chỗ trống | **25 / 598** | **2** (`Môc` → `Cốc`, `Bhận` → `Chân`) |

Chỗ chí tử là **điểm của lối cả câu không dùng làm độ tin cậy được**. Mấy phép
sửa nhầm lại đứng đầu bảng điểm — `zợi` → `sợi` hơn +0,504, `ghứ` → `chữ` hơn
+0,715, đều là giọng Pháp của Fleur mà người dịch cố ý viết chệch — trong khi
phép sửa thật chỉ quanh +0,10. Thứ tự sai hướng thì không ngưỡng nào tách được
hai loại: nâng ngưỡng lên là mất phép sửa thật trước khi chặn được phép sửa nhầm.
Đo được trên bộ 12 ca bẫy: lối cả câu đạt 7/12 chỉ trong khoảng ngưỡng 0,01…0,06
rồi tụt còn 3/12 ở ngưỡng 0,10, còn lối chỗ trống giữ 6…7/12 suốt từ 0,01 tới
0,25 — rộng gấp hai mươi lần.

Và nó bắt được `chia xẻ` → `chia sẻ`, đúng lớp lỗi mà cả tầng mô hình tồn tại để
giải: hai bên đều là tiếng Việt trôi chảy nên phải hiểu câu mới phân được. Lối cả
câu bỏ sót đúng ca ấy.

**Mặc định vẫn là lối cả câu**, vì đỉnh của hai lối bằng nhau mà lối kia chậm hơn
1,7 lần trên một cuốn và 2,6 lần trên bộ truyện dài đoạn — 37 phút thay cho 14
phút. Đáng đổi khi nào ngưỡng phải chạy theo mô hình khác hoặc loại sách khác, vì
lúc ấy một điểm số xếp đúng chiều mới đáng cái giá thời gian.

Ba chỗ dễ vấp nếu đụng vào lối điền chỗ trống:

- **Ngưỡng gắn với lối chấm.** Hai lối chia điểm cho hai khoảng dài khác nhau nên
  thang đo khác nhau (biên độ trung vị 0,059 so với 0,076). Đổi lối mà giữ ngưỡng
  của lối kia thì van an toàn lệch chừng 1,3 lần. Con số 0,018 là **khe giữa hai
  cuốn sách**: phép sửa nhầm có biên độ cao nhất là `zăc` → `ắc` (+0,014), phép
  sửa đúng có biên độ thấp nhất mà ta cần là `đựoc` → `được` (+0,021). Hạ thấp hơn
  thì Phàm Nhân Tu Tiên được thêm 9 phép đúng nhưng Harry Potter bắt đầu phá tên
  phiên âm — hai cuốn không hoà nhau được ở đấy, nên lấy đầu chặt.
- **Đuôi càng dài càng đúng**, ngược với trực giác "đuôi nằm ở mẫu số nên nó pha
  loãng chênh lệch". Đo được: 60 byte → 5/12 ca đúng, 180 → 6/12, 400 → 7/12.
  Loãng thì có thật, nhưng nó nén đúng mấy ca mô hình tự tin sai.
- **Ngữ cảnh phải lấy từ hai đoạn kề bên**, không bó trong đoạn đang soát. Tiểu
  thuyết đầy đoạn một câu thoại, nên "hai câu trước" mà bó trong đoạn thì phần
  lớn trường hợp chẳng có câu nào.

Mô hình vẫn **không sinh ra chữ nào**: nó chấm những chữ tầng luật đưa cho nó
điền vào chỗ trống, chứ không được tự chọn chữ ngoài danh sách ứng viên.

```bash
cargo run --release -p mohinh --example so_loi_cham -- mo-hinh.gguf sach.epub
```

Công cụ ấy so hai lối trên những ca bẫy lấy ra từ chính cuốn sách, và **kiểm bộ
đệm tiền tố trước khi in con số nào** — đệm sai thì ứng viên bị chấm trong một
ngữ cảnh khác ngữ cảnh ta tưởng, chương trình vẫn chạy trơn và vẫn in ra những
con số trông hợp lý.

### Tầng luật là mặc định, mô hình chỉ **lật ngược** khi hơn rõ

Ứng viên đầu bảng do tầng luật xếp; mô hình phải chấm một ứng viên khác hơn **nó**
quá ngưỡng thì mới được đổi — chứ không phải hơn bản gốc. So với bản gốc là so
với một chuỗi vô nghĩa nên ứng viên nào cũng thắng, và việc chọn rơi hết vào nhiễu.

## Cạm bẫy

**Dựng lại NFC trước khi cắt từ.** Dấu tổ hợp không phải chữ cái, nên chữ gõ rời
bị cắt đôi ngay giữa từ: `Huyền` ra thành `Huyê` + `n`. Đã vấp **hai lần** — một
lần ở công cụ soi `soi_van.rs` (291 lần `tâ` đứng đầu bảng, che mất lỗ hổng
thật), một lần ở bộ gom tên riêng (bảng đầy mảnh chữ cụt). `soat::soat` đã làm
đúng thứ tự; mọi chỗ **khác** đọc `Doan::chu` đều phải tự dựng lại NFC.

**Chấm điểm chia cho số ký tự, không chia cho số token.** Thêm một token dễ đoán
có thể *nâng* điểm trung bình lên, nên mô hình từng chấm `Hắn phảii đi ngay.` cao
hơn `Hắn phải đi ngay.` — câu sai thắng câu đúng. Ngưỡng tính bằng nats/**ký tự**
nên con số nhỏ hơn hẳn mức quen thuộc (mặc định 0,03).

**Chữ cái riêng của tiếng Việt không đối xứng với chữ trơn.** Mỗi chữ `đ ă â ê ô
ơ ư` đều phải gõ thêm một phím mới ra được (`dd`, `aw`, `aa`, `ee`, `oo`, `ow`,
`uw`). Người ta **quên** những phím ấy suốt, chứ không vô tình bấm thêm — nên một
chữ đã mang dấu phụ là chữ người viết cố ý gõ ra. Vì thế **thêm** dấu phụ vào chữ
trơn (2) rẻ hơn **đụng vào** chữ đã có dấu phụ (3).

Nhưng lập luận ấy chỉ biện minh được đúng **một** chiều, và đây là chỗ dễ trượt:
nó **không** nói `ă` nên thành `â` hơn là thành `a`. `ă` và `â` là hai chữ khác
nhau, không phải hai biến thể của nhau, nên đổi sang `â` cũng là đổi sang một chữ
khác y như đổi sang `a` — hai cách phải cùng giá. Nói cách khác, cái được ưu tiên
là **giữ nguyên chữ đã mang dấu phụ**, không phải "giữ lấy một dấu phụ nào đó":
`Duơng` cho ra `Dương` (chỉ thêm dấu vào `u`, để `ơ` yên) đứng trên `Duông`, còn
`măy` thì `may` với `mây` hoà giá — chọn giữa hai chữ khác nhau là việc của tầng
từ ghép, và trong `Trò măy mắn lắm đó` thì `may mắn` có trong từ điển còn `mây
mắn` thì không.

`đ` bị **chặn hẳn** (`đ` và `d` là hai phụ âm khác nhau). Mấy nguyên âm thì chỉ
**đắt hơn**, không chặn, vì lớp lỗi "dấu móc rơi vào chữ bên cạnh" cần bỏ dấu phụ
mới sửa được: `thưộc` → `thuộc`, `xưống` → `xuống`, `cưồng` → `cuồng`, `Đạơ` →
`Đạo`. Chặn hẳn thì mất quá nửa trong 22 phép sửa loại này, đo trên hai bộ truyện.

Đặt giá thì đo được trên Phàm Nhân Tu Tiên: số phép **bỏ dấu phụ** xuống từ 18 còn
14, và bốn cái mất đi sai cả bốn — `tôc` → `to`, `măt` → `ma`, `chơt` → `choa`,
`lăo` → `lai` (cái cuối giờ ra `lão`), không sinh thêm phép bỏ dấu phụ nào. Kèm
theo là mấy chỗ đổi hướng đáng kể: `ớc` → `ước` (11 chỗ, trước ra `cổ`), `Nguời` →
`Người` (trước ra `Ngươi`), `trơnựg` → `trọng`, và 14 chỗ `ơng` → `ôn` biến mất.

**Giá của một phép sửa phải phản ánh "giữ được bao nhiêu thứ người ta đã gõ".**
Bỏ một ký tự lặp (1) < thêm dấu phụ (2) < tách chữ (3) = đụng vào chữ đã có dấu
phụ (3) < đảo hai chữ (4) < xoá/chèn/thay (6) < thay nguyên âm (8). Cộng 1 nếu số
chữ đổi. Không phân ra thì các cách hoà giá nhau và ai thắng là do **thứ tự bảng
chữ cái** — `khôgn` ra `khôn`, `đing` ra `đang`.

**Hai ký tự giống hệt nhau đứng cạnh nhau gần như chắc chắn là bấm hai lần một
phím.** Tiếng Việt không có cặp lặp, trừ `oo` trong `xoong`, `boong`, `coong`. Nên
`khôngg`, `phảii`, `mộtt`, `nhưnng` sửa được mà không cần cân nhắc cách nào khác.
Hai chỗ phải chừa:

- **`ó` là `o` mang dấu thanh**, không phải chữ cái khác — nên `xoóng` cũng là `oo`
  và cũng được che. Phép này chạy trên bộ khung đã bỏ dấu thanh nên đúng sẵn.
- **Chữ mang dấu phụ thì để tầng dấu phụ lo.** `ưu` là vần rất phổ biến (`lưu`,
  `hưu`, `mưu`, `bưu`), nên `lưư` gần như chắc chắn là `lưu` gõ trượt — chữ trơn
  bên cạnh bị ăn dấu theo — chứ không phải `lư` bấm hai lần. Xếp phép bỏ lặp rẻ hơn
  thì `lư` thắng, và thắng sai.

**Đụng vào giá thì phải nhớ `MAX_UNG_VIEN`.** `ung_vien::sinh` cắt danh sách
**theo giá** ở con số 40, mà bằng chứng mạnh nhất — từ ghép trong từ điển — thì
tầng trên mới hỏi. Nâng giá "mất dấu phụ" lên 5 là đẩy `cửa` (vừa thêm một dấu
phụ vừa mất một dấu phụ, giá 11) ra khỏi cửa sổ, và `màn cẳu sổ` ra `màn của sổ`
trên cả cuốn sách mà không tầng nào báo gì.

**Nới `MAX_UNG_VIEN` không phải cách chữa.** Thử 80 thì số lỗi bắt được *tăng*
(127 so với 126) mà chất lượng *tụt*: `ẩnh` → `ăn` thay vì `ảnh`, `kó` → `tớ` ở
hai trong ba chỗ, `Bàc` → `Báo` thay vì `Bạn`. Phần lớn ứng viên mới cũng có
trong từ điển nên chúng lọt tới tay mô hình và cho nó thêm đường chọn sai. Chỗ
nghẽn là **tầng chọn**, không phải bộ sinh ứng viên.

**Thanh gốc của chữ không dấu là thanh ngang, không phải "không có".** Để `None`
thì mọi thanh đều bị tính là khác thanh gốc và `đnag` cho ra `đang`, `đàng`,
`đáng` hoà nhau hết.

**Phông đi kèm egui không có chữ Việt.** Nó thiếu khối Latin Extended Additional
(U+1EA0–U+1EF9) — hơn nửa số chữ có dấu. `main.rs` mượn phông hệ điều hành.
`tests/phong_chu.rs` quét **mọi ký tự ngoài ASCII trong chuỗi của mã nguồn giao
diện** và kiểm phông có glyph — đã bắt được `▸` hiện ra ô vuông trống. Đừng dùng
ký hiệu ngoài Latin-1 trong chuỗi giao diện.

**Đừng gọi bất cứ thứ gì trong `mohinh` khi chưa kiểm `mohinh::du_dll()`.** Bản
dựng Windows cho `cublas64_*.dll` **nạp trễ** (`giaodien/build.rs`), nên ứng dụng
mở được trên máy chưa cài CUDA và tự mời tải về — xem `giaodien/tai_cuda.rs`. Cái
giá là hàm llama.cpp đầu tiên được gọi lúc thiếu DLL sẽ **giết tiến trình ngay tại
chỗ**: không panic, không lỗi, không dòng nhật ký nào. `card_dung_duoc`,
`liet_ke_thiet_bi` và `MoHinh::nap` đều đã tự chặn; thêm hàm công khai nào nữa thì
chặn y hệt.

Vì sao phải nạp trễ thay vì đóng gói kèm: ba DLL ấy nặng **493 MB** —
`cublasLt64_13.dll` một mình 442 MB — nên bản cài sẽ gần 600 MB cho một thứ mà
người không có card NVIDIA chẳng dùng tới. Nạp trễ rồi thì bản cài còn 43 MB, và
người cần mô hình tải thêm 375 MB từ kho `redist` của NVIDIA.

**Bản cài đặt vào thư mục người dùng, không vào Program Files.** `dong-goi/*.iss`
đặt `PrivilegesRequired=lowest` và `{localappdata}\Programs`, vì bộ tải ghi DLL
xuống **cạnh chính file exe** — vào Program Files thì tiến trình không có quyền
quản trị không ghi nổi, và tính năng tải tự hỏng.

**Sửa `du-lieu/*.txt` bằng tay là sai.** `am-tiet.txt` và `tu-ghep.txt` sinh ra
từ `crates/chinhta/examples/dung_tu_dien.rs`; sửa tay thì lần dựng lại sau mất
hết. Xem `du-lieu/NGUON.md`.

**Thêm luật mới thì đo lại bằng `soi_van.rs`.** Nó quét chín triệu tiếng trong
sách thật và liệt kê những tiếng bị coi là sai, xếp theo số lần gặp — mục nào lặp
lại nhiều lần mà bị báo sai thì gần như chắc chắn là lỗi của ta, không phải của
sách.

```bash
cargo run --release -p giaodien --example soi_van -- sach1.epub sach2.epub
```

## Số đo hiện tại

Harry Potter tập 4 (1,2 triệu chữ, 9.577 đoạn):

| | Không mô hình | Có mô hình 9B |
|---|---|---|
| Lỗi chữ nghĩa bắt được | 75 | 125 |
| Chỗ ngờ để lại | 86 | 0 |
| Thời gian | 4,4 giây | 52 giây |

Phàm Nhân Tu Tiên (25,6 triệu chữ, 162.772 đoạn, 2.467 file): 2.376 lỗi chữ nghĩa,
0 chỗ ngờ, 449 giây.

Con số đáng nhìn không phải số lỗi mà là **ai quyết**: trên Phàm Nhân Tu Tiên, các
tầng luật quyết 1.282 chỗ và mô hình chỉ quyết 332. Tầng luật càng khoẻ thì mô hình
càng ít việc, và đó cũng là lý do bộ dò nhanh gấp đôi bản chỉ có mô hình.

Cả hai cột đo ở lối chấm mặc định — chấm cả câu, ngưỡng 0,03. Lối điền chỗ trống
(`--cho-trong`, ngưỡng 0,018) bắt 125 lỗi trên Harry Potter trong 114 giây và
2.276 lỗi trên Phàm Nhân trong 37 phút; xem mục "Hai lối hỏi mô hình" ở trên.

Bộ 13 câu mẫu (dính chữ, lỗi gõ, và các ca bẫy không được sửa): **13/13**.

Bộ 12 ca bẫy lấy từ chính sách thật, so hai lối chấm và quét ngưỡng:

```bash
cargo run --release -p mohinh --example so_loi_cham -- mo-hinh.gguf sach.epub
```

Nó **kiểm bộ đệm tiền tố trước khi in con số nào**: đệm sai thì ứng viên bị chấm
trong một ngữ cảnh khác ngữ cảnh ta tưởng, mà chương trình vẫn chạy trơn và vẫn
in ra những con số trông hợp lý.

Con số "lỗi chữ nghĩa" **không** tính khoảng trắng, Unicode, dấu câu và kiểu đặt
dấu. Gộp chúng vào thì cuốn nào cũng "hàng chục nghìn lỗi" mà phần lớn là rác
định dạng — và `hoá`/`hóa` thì cả hai đều đúng.

## Bản quyền

`du-lieu/am-tiet.txt` và `du-lieu/tu-ghep.txt` chắt ra từ ba bộ từ điển của người
khác (tudientv, Wiktionary tiếng Việt, Hồ Ngọc Đức) — xem `du-lieu/NGUON.md`.
File từ điển gốc không nằm trong repo.
