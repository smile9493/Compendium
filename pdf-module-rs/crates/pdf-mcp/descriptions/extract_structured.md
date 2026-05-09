从 PDF 文件中提取结构化数据，包含页码、位置、边界框等元信息。

## 适用场景

- 需要精确定位文本位置（如高亮、标注）
- 按页处理文档（如分页分析）
- 需要文本坐标信息（如 OCR 后处理）

## 返回数据结构

```json
{
  "pages": [
    {
      "page_number": 1,
      "text": "页面完整文本",
      "bbox": [x0, y0, x1, y1],
      "lines": [
        {
          "text": "行文本",
          "bbox": [x0, y0, x1, y1]
        }
      ]
    }
  ],
  "total_pages": 10,
  "extraction_time_ms": 150
}
```

## 性能特性

- 比 `extract_text` 慢约 2-3 倍（需要计算位置信息）
- 结果会被缓存

## 限制

- 边界框精度取决于 PDF 内部结构
- 某些引擎（如 pdf-extract）可能不提供精确的 bbox
