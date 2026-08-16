# VieSpelling

Ứng dụng máy tính để bàn: kiểm và sửa lỗi chính tả tiếng Việt trong file EPUB.
Chạy một lượt, ghi ra sách đã sửa cùng một bản báo cáo HTML liệt kê từng chỗ đổi
và vì sao đổi. **Bản gốc không bao giờ bị ghi đè.**

Viết bằng Rust. Giao diện egui, mô hình ngôn ngữ chạy trên máy qua llama.cpp,
không gửi gì ra mạng.

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
| Hai tiếng dính liền | `Phúlần`→`Phú lần`, `HuyềnVũ`→`Huyền Vũ`, `erằng`→`e rằng` | luôn |
| Tiếng sai, hàng xóm quyết được | `chúg ta`→`chúng ta`, `tình thuơng`→`tình thương` | luôn |
| Tiếng sai, không ai quyết được | `Ừ thuơng à`, `ngồiở` | cần mô hình |
| Cặp dễ nhầm, tuỳ nghĩa | `dành`/`giành`, `chia sẻ`/`chia xẻ` | cần mô hình |

Hai dòng cuối cần **hiểu câu** mới phân được. Không có mô hình thì chúng nằm
nguyên trong mục *chỗ ngờ* của báo cáo chứ không bị đoán bừa.

## Nó cố ý KHÔNG sửa gì

Phần này quan trọng ngang phần trên. Một bộ sửa tự động mà hăng quá thì phá sách
nhanh hơn là chữa.

- **Số kiểu Việt Nam.** `1,5 triệu` và `12.000 đồng` giữ nguyên. Dấu phẩy là dấu
  thập phân, dấu chấm là phân nhóm hàng nghìn — ngược với tiếng Anh.
- **Từ mượn viết theo âm Việt.** `bêtông`, `micrô`, `pittông`, `rađa`, `nilông`
  không ghép được từ âm đầu + vần nào, nhưng chúng là tiếng Việt thật. Đo được
  1.813 âm tiết loại này.
- **Tên riêng của chính cuốn sách.** `Kông` trong `Hồng Kông` không có trong từ
  điển và phạm luật chính tả, nhưng nó lặp lại nhiều lần và viết hoa giữa câu —
  ứng dụng đếm cả sách để nhận ra.
- **Tên riêng nước ngoài.** `Dumbledore`, `Voldemort` không mang dấu tiếng Việt.
  Chữ không dấu nói chung mặc định không kiểm, vì không phân được `khong`
  (thiếu dấu) với `window` (tiếng Anh).
- **Cách viết hoa.** `THầy`, `KHông` trong sách quét lại là dấu tích của chữ cái
  to đầu đoạn, không phải lỗi chính tả.
- **Chữ bị thẻ HTML cắt ngang.** `khô<i>ng</i>` sửa được thì phải xén thẻ. Những
  chỗ ấy được đếm và ghi rõ trong báo cáo.
- **Nội dung trong `<pre>`, `<code>`, `<script>`, `<style>`.**
- **File không phải Unicode.** Sách còn ở bảng mã TCVN3/VNI đi qua nguyên vẹn.
  Nhận diện bảng mã là đoán, mà đoán sai thì ánh xạ hỏng toàn bộ ký tự của cả
  cuốn sách — nặng hơn mọi lỗi công cụ này chữa.

## Cách nó quyết định

### Ba tầng xác nhận một tiếng

1. **Từ điển** — 9.550 âm tiết. Có thì thôi, không xét tiếp.
2. **Cấu tạo âm tiết** — âm đầu + vần + thanh. Đỡ cho những gì từ điển thiếu.
3. Trượt cả hai thì mới sinh cách sửa, rồi **từ ghép** (69.893 mục) chọn giữa
   chúng: `chúg ta` thì `chúng ta` có trong từ điển còn `chừ ta` thì không.

Thứ tự ấy không đảo được. Chỉ dùng bảng vần thì 1.813 từ mượn bị sửa hỏng; chỉ
dùng từ điển thì mọi chữ lạ hợp lệ đều bị bắt, mà `híc` chẳng có trong bộ nào.
Xem `du-lieu/NGUON.md`.

### Chữ dính xét trước chữ sai

Sách convert từ PDF hoặc bị bóc thẻ HTML hay nuốt mất khoảng trắng — đo được 231
chỗ trong một bộ truyện. Lớp lỗi này khác mọi lớp khác ở chỗ **không chữ cái nào
sai**, chỉ thiếu một khoảng trắng.

Lưới chặn: bỏ cách chia nào có **nguyên âm lặp ngay chỗ ngắt**. Hai nguyên âm
cùng chữ nền hai bên chỗ ngắt thì đó không phải hai tiếng dính nhau mà là một
nguyên âm bị gõ hai lần — `Ngooại` là `ngoại` thừa chữ o, `phảii` là `phải` thừa
chữ i. Tách ra thì được hai chữ đều có trong từ điển mà câu thành vô nghĩa: kiểu
hỏng khó thấy nhất, vì bản sửa trông vẫn đúng tiếng Việt.

### Mô hình ngôn ngữ không được sinh ra chữ nào

Nó chỉ chấm điểm câu. Các cách sửa do tầng luật và tầng cấu tạo âm tiết sinh ra —
chúng bảo đảm mọi ứng viên đều là tiếng Việt viết đúng — còn mô hình chỉ **xếp
hạng** chúng. Cách này chặn hẳn việc mô hình viết lại văn của tác giả hay bịa tên
riêng, và kết quả lặp lại được nên kiểm thử được.

Và tầng luật là **mặc định**: ứng viên đầu bảng do luật xếp, mô hình phải chấm
một ứng viên khác hơn **nó** quá ngưỡng thì mới được lật ngược — chứ không phải
hơn bản gốc. So với bản gốc là so với một chuỗi vô nghĩa nên ứng viên nào cũng
thắng, và việc chọn rơi hết vào nhiễu.

Bằng chứng từ ghép đặt **trước** mô hình, vì nó là sự thật về tiếng Việt chứ
không phải một ước lượng: đo trên một cuốn sách thì mô hình 9 tỷ tham số chọn sai
khoảng 40% số ca thuộc loại này, mà từ ghép phân được ngay.

## Mô hình

Không bắt buộc. Nhận file GGUF bất kỳ; giao diện tự quét `C:\Dev\models` nên chọn
bằng ô thả xuống.

| Mô hình | Cỡ | Dùng khi |
|---|---|---|
| Qwen3.5-9B Q4_K_M | 5,3 GB | mặc định — phán ngữ cảnh tốt hơn |
| Qwen3.5-4B Q5_K_M | 3,1 GB | nhanh hơn, đủ cho lỗi gõ rõ ràng |

Cỡ mô hình ảnh hưởng **không đều**: chọn `thuơng`/`thương` thì mô hình nào cũng
làm được vì chênh lệch điểm rất lớn; còn chọn `để dành`/`để giành` thì cả hai đều
là tiếng Việt trôi chảy, chênh lệch nhỏ, và mô hình nhỏ hay chấm sai hướng.

### Bắt buộc chạy trên GPU

Mô hình chạy trên card đồ hoạ rời, **không có đường lùi về CPU**. Không thấy card
thì báo lỗi chứ không chạy chậm trong im lặng — chấm 9 tỷ tham số bằng CPU chậm
hơn hàng chục lần, và người dùng sẽ ngồi nhìn thanh tiến trình hàng giờ mà không
hiểu vì sao.

Muốn chạy không cần card thì bỏ chọn mô hình: ứng dụng sửa được mọi dòng "luôn"
trong bảng trên, mất khoảng 4 giây một cuốn.

## Đã đo trên sách thật

Harry Potter tập 4 — 1,2 triệu chữ, 9.577 đoạn:

| | Không mô hình | Có mô hình 9B |
|---|---|---|
| Lỗi chữ nghĩa bắt được | 75 | 126 |
| Chỗ ngờ để lại | 86 | 0 |
| Thời gian | 4,4 giây | ~60 giây |

Hai bộ truyện dài (20,4 và 25,7 triệu chữ) chạy được ở cùng quy mô.

Con số "lỗi chữ nghĩa" **không** tính khoảng trắng, Unicode, dấu câu và kiểu đặt
dấu — gộp chúng vào thì cuốn nào cũng "hàng chục nghìn lỗi" mà phần lớn là rác
định dạng, và `hoá`/`hóa` thì cả hai đều đúng.

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

## Dựng

Cần **Visual Studio Build Tools** (workload C++), **cmake**, **`LIBCLANG_PATH`**
trỏ vào thư mục chứa `libclang.dll`, và **CUDA Toolkit ≥ 12.8**.

```bash
cargo build --release
cargo test --release --workspace
```

`cuda-arch.cmake` ghim kiến trúc CUDA cần dựng — **đổi card thì sửa file ấy**,
không thì llama.cpp dựng cho cả bảy đời card. Dựng không cần CUDA:
`cargo build --release --no-default-features`.

Xem `CLAUDE.md` cho các cạm bẫy của bản dựng và quy ước của repo.

## Cấu trúc

```
crates/chinhta/    lõi ngôn ngữ — không phụ thuộc EPUB hay llama.cpp
crates/sach/       đọc/ghi EPUB giữ nguyên phần không sửa
crates/mohinh/     chấm điểm câu bằng llama.cpp trên GPU
crates/giaodien/   ứng dụng (cửa sổ + dòng lệnh)
du-lieu/           từ điển đã chắt, bảng cặp dễ nhầm
```

## Giấy phép

Mã nguồn: MIT.

`du-lieu/am-tiet.txt` và `du-lieu/tu-ghep.txt` chắt ra từ ba bộ từ điển tiếng
Việt của người khác — xem `du-lieu/NGUON.md`. Chúng không phải tác phẩm của dự án
này và không chịu giấy phép MIT ở trên.
