/**
 * MiniMax MCP Plugin
 * 提供网络搜索和图片理解功能
 */

const axios = require('axios');
const FormData = require('form-data');
const fs = require('fs');
const path = require('path');

class MiniMaxMCPPlugin {
  constructor(config = {}) {
    this.apiKey = config.api_key || process.env.MINIMAX_API_KEY;
    this.baseUrl = config.base_url || process.env.MINIMAX_BASE_URL || 'https://api.minimax.chat';
    this.webSearchEnabled = config.web_search_enabled !== false;
    this.imageUnderstandEnabled = config.image_understand_enabled !== false;
    this.timeout = config.timeout || 30000;
    this.maxImageSize = (config.max_image_size || 20) * 1024 * 1024; // 转换为字节

    if (!this.apiKey) {
      throw new Error('MiniMax API Key is required');
    }

    this.client = axios.create({
      baseURL: this.baseUrl,
      timeout: this.timeout,
      headers: {
        'Authorization': `Bearer ${this.apiKey}`,
        'Content-Type': 'application/json'
      }
    });
  }

  /**
   * 获取可用工具列表
   */
  getTools() {
    const tools = [];

    if (this.webSearchEnabled) {
      tools.push({
        name: 'web_search',
        description: '网络搜索工具,根据查询词返回搜索结果',
        parameters: {
          type: 'object',
          properties: {
            query: {
              type: 'string',
              description: '搜索查询词'
            }
          },
          required: ['query']
        }
      });
    }

    if (this.imageUnderstandEnabled) {
      tools.push({
        name: 'understand_image',
        description: '图片理解工具,对图片进行理解和分析',
        parameters: {
          type: 'object',
          properties: {
            prompt: {
              type: 'string',
              description: '对图片的提问或分析要求'
            },
            image_url: {
              type: 'string',
              description: '图片来源URL或本地路径'
            }
          },
          required: ['prompt', 'image_url']
        }
      });
    }

    return tools;
  }

  /**
   * 执行工具调用
   */
  async execute(toolName, parameters) {
    switch (toolName) {
      case 'web_search':
        return await this.webSearch(parameters.query);
      case 'understand_image':
        return await this.understandImage(parameters.prompt, parameters.image_url);
      default:
        throw new Error(`Unknown tool: ${toolName}`);
    }
  }

  /**
   * 网络搜索
   * @param {string} query - 搜索查询词
   * @returns {Promise<Object>} 搜索结果
   */
  async webSearch(query) {
    if (!this.webSearchEnabled) {
      throw new Error('Web search is disabled');
    }

    try {
      const response = await this.client.post('/v1/web_search', {
        query: query
      });

      return {
        success: true,
        results: response.data.results || [],
        related_searches: response.data.related_searches || [],
        query: query
      };
    } catch (error) {
      return {
        success: false,
        error: error.message,
        query: query
      };
    }
  }

  /**
   * 图片理解
   * @param {string} prompt - 对图片的提问
   * @param {string} imageUrl - 图片URL或本地路径
   * @returns {Promise<Object>} 分析结果
   */
  async understandImage(prompt, imageUrl) {
    if (!this.imageUnderstandEnabled) {
      throw new Error('Image understanding is disabled');
    }

    try {
      let imageData;
      let isLocalFile = false;

      // 判断是URL还是本地文件
      if (imageUrl.startsWith('http://') || imageUrl.startsWith('https://')) {
        // 远程URL - 下载图片
        const response = await axios.get(imageUrl, {
          responseType: 'arraybuffer',
          maxContentLength: this.maxImageSize
        });
        imageData = Buffer.from(response.data);
      } else {
        // 本地文件
        isLocalFile = true;
        const filePath = path.resolve(imageUrl);

        if (!fs.existsSync(filePath)) {
          throw new Error(`File not found: ${filePath}`);
        }

        const stats = fs.statSync(filePath);
        if (stats.size > this.maxImageSize) {
          throw new Error(`File size exceeds limit: ${stats.size} > ${this.maxImageSize}`);
        }

        imageData = fs.readFileSync(filePath);
      }

      // 使用FormData上传图片
      const formData = new FormData();
      formData.append('prompt', prompt);
      formData.append('image', imageData, {
        filename: isLocalFile ? path.basename(imageUrl) : 'image.jpg',
        contentType: 'image/jpeg'
      });

      const response = await this.client.post('/v1/understand_image', formData, {
        headers: {
          ...formData.getHeaders()
        }
      });

      return {
        success: true,
        analysis: response.data.analysis || response.data.result,
        metadata: {
          prompt: prompt,
          image_url: imageUrl,
          is_local: isLocalFile,
          format: this.detectImageFormat(imageData),
          size: imageData.length
        }
      };
    } catch (error) {
      return {
        success: false,
        error: error.message,
        prompt: prompt,
        image_url: imageUrl
      };
    }
  }

  /**
   * 检测图片格式
   */
  detectImageFormat(buffer) {
    if (buffer.length < 4) return 'unknown';

    // PNG
    if (buffer[0] === 0x89 && buffer[1] === 0x50 && buffer[2] === 0x4E && buffer[3] === 0x47) {
      return 'PNG';
    }
    // JPEG
    if (buffer[0] === 0xFF && buffer[1] === 0xD8 && buffer[2] === 0xFF) {
      return 'JPEG';
    }
    // GIF
    if (buffer[0] === 0x47 && buffer[1] === 0x49 && buffer[2] === 0x46) {
      return 'GIF';
    }
    // WebP
    if (buffer[0] === 0x52 && buffer[1] === 0x49 && buffer[2] === 0x46 && buffer[3] === 0x46) {
      return 'WebP';
    }

    return 'unknown';
  }

  /**
   * 健康检查
   */
  async healthCheck() {
    try {
      const response = await this.client.get('/health');
      return response.status === 200;
    } catch {
      return false;
    }
  }
}

module.exports = MiniMaxMCPPlugin;
