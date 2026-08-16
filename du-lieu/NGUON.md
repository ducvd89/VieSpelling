# Dữ liệu trong thư mục này

## `am-tiet.txt` và `tu-ghep.txt`

Chắt ra từ ba bộ từ điển tiếng Việt, bằng `crates/chinhta/examples/dung_tu_dien.rs`:

| Nguồn | Mục | Ghi chú |
|---|---|---|
| tudientv | 36.534 | |
| Wiktionary tiếng Việt | 32.484 | có lẫn vài chục trang bản mẫu, đã lọc |
| Hồ Ngọc Đức | 73.901 | rộng nhất, và nhiễu nhất |

Kết quả: **9.550 âm tiết**, **69.893 từ ghép**.

File từ điển gốc **không** nằm trong repo — chúng là dữ liệu của người khác, tổng
gần 7 MB, và ứng dụng chỉ cần phần đã chắt. Dựng lại:

```bash
cargo run --release -p chinhta --example dung_tu_dien -- tudientvdict.txt wikination.txt hongocducdict.txt
```

### Đã lọc gì khỏi từ điển

Cả ba nguồn đều có lỗi chính tả lẫn bên trong — `thuơng`, `lưòi`, `ngườì`,
`dướỉ`, `khảch`, `gĩữ`. Để nguyên thì tai hại hơn hẳn bình thường: từ điển là
phép kiểm **chính** của ứng dụng, nên một lỗi nằm trong đó là một lỗi ứng dụng
vĩnh viễn không bắt được nữa.

Phép lọc: loại mục nào **sai cấu tạo âm tiết**, **có mang dấu tiếng Việt**, và
**sinh ra được cách sửa giữ nguyên bộ khung chữ cái**. Ba điều kiện phải đủ cả.

Bỏ bớt điều kiện nào cũng hỏng, và đã hỏng thật khi thử:

- Bỏ điều kiện "giữ nguyên khung chữ" thì loại nhầm 607 mục, gồm cả từ mượn thật:
  `alô` bị coi là sai vì "sửa" được thành `lô`, `axit` thành `xít`, `balô` thành
  `bảo`.
- Bỏ điều kiện "có dấu tiếng Việt" thì loại cả viết tắt và ký hiệu — `atm`,
  `adn`, `abc`.
- Đếm số nguồn chứa mục ấy thì **không** phân được gì: `bêtông` (từ thật) và
  `thuơng` (lỗi) đều chỉ xuất hiện ở đúng một nguồn.

Với phép lọc đủ ba điều kiện thì chỉ 60 mục bị loại, và mục nào cũng hỏng thật.

### Vì sao cần kho âm tiết khi đã có bảng vần

Bảng vần trong `am_tiet.rs` mô hình hoá ngữ âm tiếng Việt, nên nó bác bỏ mọi thứ
nằm ngoài hệ thống ấy — kể cả từ mượn viết theo âm Việt, vốn là tiếng Việt thật
và gặp thường xuyên: `bêtông`, `micrô`, `pittông`, `rađa`, `nilông`, `cafê`.
Đo được **1.813 âm tiết** trong từ điển bị bảng vần bác bỏ.

Mỗi mục bị bác oan không phải là bỏ sót — ứng dụng tự sửa, nên đó là một chữ
**đúng** bị đổi thành chữ **sai**.

Chiều ngược lại cũng có: `híc` không có trong bộ từ điển nào cả. Nên hai phép
kiểm bù cho nhau, và một tiếng chỉ bị bắt khi **trượt cả hai**.

## `de-nham.txt`

Gõ tay. Xem phần đầu file.
