# VieSpellcheck

Ứng dụng máy tính để bàn: kiểm và sửa lỗi chính tả tiếng Việt trong file EPUB.
Chạy một lượt, ghi ra sách đã sửa cùng một bản báo cáo HTML liệt kê từng chỗ đổi
và vì sao đổi. **Bản gốc không bao giờ bị ghi đè.**

```bash
cargo run --release -p giaodien --bin vie-spellcheck
```

## Nó sửa những gì

Chia tầng theo **độ chắc chắn**, vì ứng dụng tự sửa rồi mới báo cáo — sửa nhầm
tốn của người dùng nhiều hơn bỏ sót.

| Tầng | Ví dụ | Tự sửa? |
|---|---|---|
| Unicode, ký tự vô hình | `khô<U+00AD>ng` → `không`, tổ hợp rời → NFC | luôn |
| Khoảng trắng, dấu câu | `nói , rồi` → `nói, rồi` | luôn |
| Dấu thanh đặt sai nguyên âm | `qúy`→`quý`, `đựơc`→`được`, `chuỵên`→`chuyện` | luôn |
| Kiểu đặt dấu | `hoà` ⇄ `hòa`, kéo về kiểu đa số của chính cuốn sách | tuỳ chọn |
| Cặp dễ nhầm, dạng luôn sai | `xử dụng`→`sử dụng`, `che dấu`→`che giấu` | luôn |
| Hai tiếng dính liền | `Phúlần`→`Phú lần`, `HuyềnVũ`→`Huyền Vũ` | luôn |
| Tiếng sai, hàng xóm quyết được | `chúg ta`→`chúng ta`, `tình thuơng`→`tình thương` | luôn |
| Tiếng sai, không ai quyết được | `Ừ thuơng à` | cần mô hình |
| Cặp dễ nhầm, tuỳ nghĩa | `dành`/`giành`, `chia sẻ`/`chia xẻ` | cần mô hình |

Hai dòng cuối cần **hiểu câu** mới phân được. Không có mô hình thì chúng nằm
nguyên trong mục *chỗ ngờ* của báo cáo chứ không bị đoán bừa.

### Ba tầng xác nhận một tiếng, theo thứ tự

1. **Từ điển** (9.550 âm tiết) — có thì thôi, không xét tiếp.
2. **Cấu tạo âm tiết** — âm đầu + vần + thanh. Đỡ cho những gì từ điển thiếu.
3. Trượt cả hai thì mới sinh cách sửa, rồi **từ ghép** (69.893 mục) chọn giữa
   chúng: `chúg ta` thì `chúng ta` có trong từ điển còn `chừ ta` thì không.

Thứ tự ấy không đảo được. Chỉ dùng bảng vần thì 1.813 từ mượn viết theo âm Việt
(`bêtông`, `micrô`, `rađa`) bị sửa hỏng. Chỉ dùng từ điển thì mọi chữ lạ hợp lệ
đều bị bắt. Xem `du-lieu/NGUON.md`.

Bằng chứng từ ghép **đặt trước mô hình ngôn ngữ**, vì nó là sự thật về tiếng
Việt chứ không phải một ước lượng: đo trên một cuốn sách thì mô hình 9 tỷ tham
số chọn sai khoảng 40% số ca thuộc loại này, mà từ ghép phân được ngay.

### Chữ dính xét trước chữ sai

Sách convert từ PDF hoặc bị bóc thẻ HTML hay nuốt mất khoảng trắng — đo được
231 chỗ trong một bộ truyện. Lớp lỗi này khác mọi lớp khác ở chỗ **không chữ cái
nào sai**, chỉ thiếu một khoảng trắng, nên phép sửa giữ nguyên từng ký tự người
ta đã gõ thay vì đoán họ định gõ gì.

Lưới chặn: mảnh từ thứ hai trở đi phải **mở đầu bằng phụ âm**. Mảnh mở đầu bằng
nguyên âm gần như luôn là một nguyên âm bị gõ lặp trong cùng một tiếng —
`Huoàng` là `Hoàng` thừa chữ u, `phảii` là `phải` thừa chữ i. Tách chúng ra thì
được hai chữ đều có trong từ điển mà câu thành vô nghĩa: kiểu hỏng khó thấy
nhất, vì bản sửa trông vẫn đúng tiếng Việt.

## Nó cố ý KHÔNG sửa gì

Đây là phần quan trọng ngang phần trên. Một bộ sửa tự động mà hăng quá thì phá
sách nhanh hơn là chữa.

- **Số kiểu Việt Nam.** `1,5 triệu` và `12.000 đồng` giữ nguyên. Dấu phẩy là dấu
  thập phân, dấu chấm là phân nhóm hàng nghìn — ngược với tiếng Anh.
- **Tên riêng nước ngoài.** `Dumbledore`, `Voldemort` không phải tiếng Việt và
  cũng không mang dấu, nên không đụng tới. Chữ không dấu nói chung mặc định
  không kiểm, vì không phân được `khong` (thiếu dấu) với `window` (tiếng Anh).
- **Cách viết hoa.** `THầy`, `KHông` trong sách quét lại là dấu tích của chữ cái
  to đầu đoạn. Không phải lỗi chính tả, không phải việc của ứng dụng này.
- **Chữ bị thẻ HTML cắt ngang.** `khô<i>ng</i>` sửa được thì phải xén thẻ, mà đó
  là đổi cách trình bày. Những chỗ ấy được đếm và ghi rõ trong báo cáo.
- **Nội dung trong `<pre>`, `<code>`, `<script>`, `<style>`.**
- **File không phải Unicode.** Sách cũ còn nằm ở bảng mã TCVN3/VNI đi qua nguyên
  vẹn, không được nhận diện cũng không được chuyển. Nhận diện bảng mã là đoán,
  mà đoán sai thì ánh xạ hỏng toàn bộ ký tự của cả cuốn sách — nặng hơn mọi lỗi
  công cụ này chữa. Chuyển bằng công cụ chuyên dụng trước, rồi mới đưa vào đây.

## Mô hình ngôn ngữ

Không bắt buộc, nhưng thiếu nó thì hai lớp lỗi cuối bảng trên không được sửa.

**Mô hình không được sinh ra chữ nào.** Nó chỉ chấm điểm: cho một câu, trả về
log-xác suất trung bình mỗi token. Các cách sửa do tầng luật và tầng cấu tạo âm
tiết sinh ra — chúng bảo đảm mọi ứng viên đều là tiếng Việt viết đúng — còn mô
hình chỉ **xếp hạng** chúng. Cách này chặn hẳn việc mô hình viết lại văn của tác
giả hay bịa tên riêng, và kết quả lặp lại được nên kiểm thử được.

Bản gốc luôn được chấm cùng và được cộng thêm một ngưỡng, tức là **bản gốc thắng
khi hoà**. Kéo ngưỡng lên là dè dặt hơn: sửa ít, sai ít.

Hai mô hình đã tải sẵn ở `C:\Dev\models\`; giao diện tự quét thư mục ấy nên chọn
bằng ô thả xuống, khỏi mở hộp thoại.

| Mô hình | Cỡ | Dùng khi |
|---|---|---|
| `qwen3.5-9b/Qwen3.5-9B-Q4_K_M.gguf` | 5,3 GB | mặc định — phán ngữ cảnh tốt hơn |
| `qwen3.5-4b/Qwen3.5-4B-Q5_K_M.gguf` | 3,1 GB | nhanh hơn, đủ cho lỗi gõ sai rõ ràng |

Cỡ mô hình ảnh hưởng **không đều** giữa hai việc: chọn `thuơng`/`thương` thì mô
hình nào cũng làm được vì chênh lệch điểm rất lớn; còn chọn `để dành`/`để giành`
thì cả hai đều là tiếng Việt trôi chảy, chênh lệch nhỏ, và mô hình nhỏ hay chấm
sai hướng — mà gặp ngưỡng an toàn thì "chấm sai hướng" thành "không sửa gì".

### Bắt buộc chạy trên GPU

Mô hình chạy trên card đồ hoạ rời, **không có đường lùi về CPU**. Không thấy card
thì ứng dụng báo lỗi chứ không chạy chậm trong im lặng — chấm 9 tỷ tham số bằng
CPU chậm hơn hàng chục lần, và người dùng sẽ ngồi nhìn thanh tiến trình hàng giờ
mà không hiểu vì sao.

Muốn chạy không cần card thì bỏ chọn mô hình trong Cài đặt: ứng dụng chạy bằng
luật, sửa được mọi dòng "luôn" trong bảng trên.

## Dựng

Cần **Visual Studio Build Tools (workload C++)**, **cmake**, và **`LIBCLANG_PATH`**
trỏ vào thư mục chứa `libclang.dll` — `llama-cpp-sys-2` dựng llama.cpp bằng cmake
và sinh binding bằng bindgen.

Bản mặc định bật CUDA nên cần thêm **CUDA Toolkit ≥ 12.8** (card Blackwell / RTX
50xx cần từ bản này trở lên). Đặt `CUDA_PATH` và cho `%CUDA_PATH%\bin` vào PATH.

```bash
cargo build --release
cargo test --workspace
```

`cuda-arch.cmake` ghim kiến trúc CUDA cần dựng — **đổi card thì sửa file ấy**,
không thì llama.cpp dựng cho cả bảy đời card và mất hàng chục phút.

Dựng không cần CUDA:

```bash
cargo build --release --no-default-features
```

Bản ấy vẫn chạy, chỉ là không nạp được mô hình.

## Bản dòng lệnh

Cùng lõi với bản cửa sổ, nên chạy nó là kiểm được đúng cái mà cửa sổ sẽ làm.

```bash
cargo run --release -p giaodien --bin vsc -- sach.epub --kho
```

`--kho` chạy hết mọi tầng rồi in báo cáo mà **không ghi file nào** — cách an toàn
để xem bộ sửa định làm gì trước khi cho nó động vào sách thật.

| Cờ | Nghĩa |
|---|---|
| `-o <file>` | file ra (mặc định `<tên> (đã sửa).epub`) |
| `-m <file.gguf>` | dùng mô hình ngôn ngữ |
| `--kho` | chạy khô, không ghi gì |
| `-v` | in từng phép sửa |

## Cấu trúc

```
crates/chinhta/    lõi ngôn ngữ — không phụ thuộc EPUB hay llama.cpp
  am_tiet.rs       cấu tạo âm tiết: âm đầu + vần + thanh
  dau_thanh.rs     vị trí dấu thanh, kiểu cũ/mới
  chuan_hoa.rs     Unicode, khoảng trắng, dấu câu
  de_nham.rs       cặp dễ nhầm (bảng ở du-lieu/de-nham.txt)
  ung_vien.rs      từ tiếng sai sinh ra các cách sửa
  doi_chieu.rs     so bản trước/sau, ra khoảng byte cần vá
  soat.rs          điều phối các tầng
crates/sach/       đọc/ghi EPUB giữ nguyên phần không sửa
crates/mohinh/     chấm điểm câu bằng llama.cpp
crates/giaodien/   ứng dụng (cửa sổ + dòng lệnh)
```

## Đã đo trên sách thật

| Sách | Chữ | Lỗi chính tả | Chạy khô |
|---|---|---|---|
| Harry Potter tập 4 | 1,2 triệu | 31 | 1,0 giây |
| Phàm Nhân Tu Tiên | 25,7 triệu | 763 | — |
| Đạo | 20,4 triệu | 1.500 | — |

Số ở cột "lỗi chính tả" **không** tính khoảng trắng, Unicode, dấu câu và kiểu đặt
dấu — gộp chúng vào thì cuốn nào cũng "hàng chục nghìn lỗi" mà phần lớn là rác
định dạng.
