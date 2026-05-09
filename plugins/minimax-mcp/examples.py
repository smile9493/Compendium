#!/usr/bin/env python3
"""
MiniMax MCP使用示例
演示如何使用网络搜索和图片理解功能
"""

import asyncio
import json
from typing import Dict, Any


class MiniMaxMCPClient:
    """MiniMax MCP客户端示例"""

    def __init__(self, api_key: str):
        self.api_key = api_key
        self.base_url = "https://api.minimax.chat"

    async def web_search(self, query: str) -> Dict[str, Any]:
        """
        网络搜索示例

        Args:
            query: 搜索查询词

        Returns:
            搜索结果
        """
        print(f"\n🔍 搜索: {query}")

        # 这里模拟MCP调用
        # 实际使用时通过MCP协议调用
        result = {
            "tool": "web_search",
            "parameters": {
                "query": query
            }
        }

        print(f"📤 发送请求: {json.dumps(result, ensure_ascii=False, indent=2)}")
        print("⏳ 等待结果...")

        # 模拟返回结果
        mock_result = {
            "success": True,
            "query": query,
            "results": [
                {
                    "title": f"关于'{query}'的相关文章1",
                    "url": "https://example.com/article1",
                    "snippet": f"这是关于{query}的详细内容..."
                },
                {
                    "title": f"关于'{query}'的相关文章2",
                    "url": "https://example.com/article2",
                    "snippet": f"深入了解{query}的最佳实践..."
                }
            ],
            "related_searches": [
                f"{query} 教程",
                f"{query} 最佳实践",
                f"{query} 示例"
            ]
        }

        print(f"✅ 搜索结果: {json.dumps(mock_result, ensure_ascii=False, indent=2)}")
        return mock_result

    async def understand_image(self, prompt: str, image_url: str) -> Dict[str, Any]:
        """
        图片理解示例

        Args:
            prompt: 对图片的提问
            image_url: 图片URL或本地路径

        Returns:
            分析结果
        """
        print(f"\n🖼️  分析图片: {image_url}")
        print(f"❓ 问题: {prompt}")

        # 这里模拟MCP调用
        result = {
            "tool": "understand_image",
            "parameters": {
                "prompt": prompt,
                "image_url": image_url
            }
        }

        print(f"📤 发送请求: {json.dumps(result, ensure_ascii=False, indent=2)}")
        print("⏳ 分析中...")

        # 模拟返回结果
        mock_result = {
            "success": True,
            "analysis": f"根据图片分析,{prompt}的答案是:这是一张包含文本和图表的图片,主要内容是...",
            "metadata": {
                "prompt": prompt,
                "image_url": image_url,
                "is_local": not image_url.startswith("http"),
                "format": "PNG",
                "size": 1024000
            }
        }

        print(f"✅ 分析结果: {json.dumps(mock_result, ensure_ascii=False, indent=2)}")
        return mock_result


async def example_web_search():
    """网络搜索示例"""
    print("\n" + "="*60)
    print("示例1: 网络搜索")
    print("="*60)

    client = MiniMaxMCPClient(api_key="your-api-key")

    # 搜索PDF相关内容
    result = await client.web_search("PDF文本提取最佳实践")

    # 使用搜索结果
    if result["success"]:
        print("\n📚 搜索结果摘要:")
        for i, item in enumerate(result["results"], 1):
            print(f"  {i}. {item['title']}")
            print(f"     {item['url']}")

        print("\n🔍 相关搜索建议:")
        for search in result["related_searches"]:
            print(f"  - {search}")


async def example_image_understand():
    """图片理解示例"""
    print("\n" + "="*60)
    print("示例2: 图片理解")
    print("="*60)

    client = MiniMaxMCPClient(api_key="your-api-key")

    # 分析远程图片
    result1 = await client.understand_image(
        prompt="请描述这张图片的内容",
        image_url="https://example.com/chart.png"
    )

    # 分析本地图片
    result2 = await client.understand_image(
        prompt="分析这个PDF页面的布局结构",
        image_url="/path/to/pdf-page.png"
    )


async def example_pdf_workflow():
    """PDF处理工作流示例"""
    print("\n" + "="*60)
    print("示例3: PDF处理工作流(结合多个工具)")
    print("="*60)

    client = MiniMaxMCPClient(api_key="your-api-key")

    print("\n📋 工作流程:")
    print("  1. 提取PDF文本")
    print("  2. 提取关键词")
    print("  3. 网络搜索相关资料")
    print("  4. 分析PDF中的图表")
    print("  5. 生成综合报告")

    # 步骤1: 提取PDF文本(使用PDF Module)
    print("\n📄 步骤1: 提取PDF文本")
    print("  使用工具: extract_text")
    print("  结果: 提取了5000字符的文本内容")

    # 步骤2: 提取关键词(使用PDF Module)
    print("\n🏷️  步骤2: 提取关键词")
    print("  使用工具: extract_keywords")
    keywords = ["PDF", "文本提取", "OCR", "布局分析"]
    print(f"  结果: {keywords}")

    # 步骤3: 网络搜索(使用MiniMax)
    print("\n🔍 步骤3: 网络搜索相关资料")
    for keyword in keywords[:2]:
        result = await client.web_search(keyword)

    # 步骤4: 分析图表(使用MiniMax)
    print("\n📊 步骤4: 分析PDF中的图表")
    result = await client.understand_image(
        prompt="分析这个图表的数据和趋势",
        image_url="/path/to/chart.png"
    )

    # 步骤5: 生成报告
    print("\n📝 步骤5: 生成综合报告")
    print("  整合PDF文本、搜索结果和图表分析")
    print("  ✅ 报告生成完成")


async def main():
    """主函数"""
    print("\n" + "="*60)
    print("MiniMax MCP 使用示例")
    print("="*60)

    # 示例1: 网络搜索
    await example_web_search()

    # 示例2: 图片理解
    await example_image_understand()

    # 示例3: PDF处理工作流
    await example_pdf_workflow()

    print("\n" + "="*60)
    print("✅ 所有示例完成")
    print("="*60)


if __name__ == "__main__":
    asyncio.run(main())
