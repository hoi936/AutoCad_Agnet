# Quy ước

Model space, WCS XY, Z=0, không xoay. Origin mặc định (0,0), domain mm.
Một closed polyline: AI_OUTLINE. Bốn circle: AI_HOLES. Dimension: AI_DIM.
Lỗ lần lượt trái dưới, phải dưới, phải trên, trái trên; offset từ hai mép gần nhất.
Radius = hole_diameter / 2. edge_offset > radius; khoảng hai tâm > đường kính.
Sai số readback 1e-7 mm ở slice mock. Chỉ sửa cả semantic plate đã chọn, không vẽ chồng.
