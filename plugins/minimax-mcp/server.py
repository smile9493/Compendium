#!/usr/bin/env python3
"""
MiniMax MCP Server
提供网络搜索和图片理解MCP工具
"""

import os
import sys
import json
import asyncio
from typing import Any, Dict, List
from mcp.server import Server
from mcp.types import Tool, TextContent
import aiohttp

# 配置
MINIMAX_API_KEY = os.getenv("MINIMAX_API_KEY")
MINIMAX_BASE_URL = os.getenv("MINIMAX_BASE_URL", "https://api.minimax.chat")
WEB_SEARCH_ENABLED = os.getenv("MINIMAX_WEB_SEARCH_ENABLED", "true").lower() == "true"
IMAGE_UNDERSTAND_ENABLED = os.getenv("MINIMAX_IMAGE_UNDERSTAND_ENABLED", "true").lower() == "true"
TIMEOUT = int(os.getenv("MINIMAX_TIMEOUT", "30"))
MAX_IMAGE_SIZE = int(os.getenv("MINIMAX_MAX_IMAGE_SIZE", "20")) * 1024 * 1024

# 创建MCP服务器
server = Server("minimax-mcp")


@server.list_tools()
async def list_tools() -> List[Tool]:
    """列出可用工具"""
    tools = []

    if WEB_SEARCH_ENABLED:
        tools.append(
            Tool(
                name="web_search",
                description="网络搜索工具,根据查询词返回搜索结果和相关建议",
                inputSchema={
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "搜索查询词"
                        }
                    },
                    "required": ["query"]
                }
            )
        )

    if IMAGE_UNDERSTAND_ENABLED:
        tools.append(
            Tool(
                name="understand_image",
                description="图片理解工具,对图片进行理解和分析,支持JPEG、PNG、GIF、WebP格式(最大20MB)",
                inputSchema={
                    "type": "object",
                    "properties": {
                        "prompt": {
                            "type": "string",
                            "description": "对图片的提问或分析要求"
                        },
                        "image_url": {
                            "type": "string",
                            "description": "图片来源,支持HTTP/HTTPS URL或本地文件路径"
                        }
                    },
                    "required": ["prompt", "image_url"]
                }
            )
        )

    return tools


@server.call_tool()
async def call_tool(name: str, arguments: Dict[str, Any]) -> List[TextContent]:
    """执行工具调用"""
    if name == "web_search":
        return await web_search(arguments["query"])
    elif name == "understand_image":
        return await understand_image(arguments["prompt"], arguments["image_url"])
    else:
        raise ValueError(f"Unknown tool: {name}")


async def web_search(query: str) -> List[TextContent]:
    """网络搜索"""
    if not MINIMAX_API_KEY:
        return [TextContent(type="text", text="Error: MINIMAX_API_KEY not configured")]

    try:
        async with aiohttp.ClientSession() as session:
            async with session.post(
                f"{MINIMAX_BASE_URL}/v1/web_search",
                headers={
                    "Authorization": f"Bearer {MINIMAX_API_KEY}",
                    "Content-Type": "application/json"
                },
                json={"query": query},
                timeout=aiohttp.ClientTimeout(total=TIMEOUT)
            ) as response:
                data = await response.json()

                result = {
                    "success": True,
                    "query": query,
                    "results": data.get("results", []),
                    "related_searches": data.get("related_searches", [])
                }

                return [TextContent(type="text", text=json.dumps(result, ensure_ascii=False, indent=2))]

    except Exception as e:
        return [TextContent(type="text", text=f"Error: {str(e)}")]


async def understand_image(prompt: str, image_url: str) -> List[TextContent]:
    """图片理解"""
    if not MINIMAX_API_KEY:
        return [TextContent(type="text", text="Error: MINIMAX_API_KEY not configured")]

    try:
        # 判断是URL还是本地文件
        if image_url.startswith("http://") or image_url.startswith("https://"):
            # 远程URL
            async with aiohttp.ClientSession() as session:
                async with session.get(image_url) as img_response:
                    image_data = await img_response.read()

                    if len(image_data) > MAX_IMAGE_SIZE:
                        return [TextContent(type="text", text=f"Error: Image size exceeds limit ({len(image_data)} > {MAX_IMAGE_SIZE})")]

                    # 发送到MiniMax API
                    form_data = aiohttp.FormData()
                    form_data.add_field("prompt", prompt)
                    form_data.add_field("image", image_data, filename="image.jpg", content_type="image/jpeg")

                    async with session.post(
                        f"{MINIMAX_BASE_URL}/v1/understand_image",
                        headers={"Authorization": f"Bearer {MINIMAX_API_KEY}"},
                        data=form_data,
                        timeout=aiohttp.ClientTimeout(total=TIMEOUT)
                    ) as response:
                        data = await response.json()

                        result = {
                            "success": True,
                            "analysis": data.get("analysis") or data.get("result"),
                            "metadata": {
                                "prompt": prompt,
                                "image_url": image_url,
                                "is_local": False,
                                "size": len(image_data)
                            }
                        }

                        return [TextContent(type="text", text=json.dumps(result, ensure_ascii=False, indent=2))]
        else:
            # 本地文件
            import pathlib
            file_path = pathlib.Path(image_url).expanduser().resolve()

            if not file_path.exists():
                return [TextContent(type="text", text=f"Error: File not found: {file_path}")]

            file_size = file_path.stat().st_size
            if file_size > MAX_IMAGE_SIZE:
                return [TextContent(type="text", text=f"Error: File size exceeds limit ({file_size} > {MAX_IMAGE_SIZE})")]

            with open(file_path, "rb") as f:
                image_data = f.read()

            async with aiohttp.ClientSession() as session:
                form_data = aiohttp.FormData()
                form_data.add_field("prompt", prompt)
                form_data.add_field("image", image_data, filename=file_path.name, content_type="image/jpeg")

                async with session.post(
                    f"{MINIMAX_BASE_URL}/v1/understand_image",
                    headers={"Authorization": f"Bearer {MINIMAX_API_KEY}"},
                    data=form_data,
                    timeout=aiohttp.ClientTimeout(total=TIMEOUT)
                ) as response:
                    data = await response.json()

                    result = {
                        "success": True,
                        "analysis": data.get("analysis") or data.get("result"),
                        "metadata": {
                            "prompt": prompt,
                            "image_url": str(file_path),
                            "is_local": True,
                            "size": file_size
                        }
                    }

                    return [TextContent(type="text", text=json.dumps(result, ensure_ascii=False, indent=2))]

    except Exception as e:
        return [TextContent(type="text", text=f"Error: {str(e)}")]


async def main():
    """主函数"""
    if not MINIMAX_API_KEY:
        print("Warning: MINIMAX_API_KEY not set", file=sys.stderr)

    async with server:
        await server.run()


if __name__ == "__main__":
    asyncio.run(main())
