# 通义千问（Qwen）API 详细文档

本文档详细说明了通义千问（Qwen）API 的完整接口规范，包括请求参数、响应格式和使用示例。

## 目录

1. [API 概述](#api-概述)
2. [请求参数详解](#请求参数详解)
3. [消息类型](#消息类型)
4. [响应格式](#响应格式)
5. [使用示例](#使用示例)
6. [特殊功能](#特殊功能)

---

## API 概述

### 基础信息

- **API 端点**: `https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation`
- **认证方式**: Bearer Token（通过 `Authorization` 头传递）
- **请求方法**: POST
- **内容类型**: `application/json`

### 支持的模型

- **通义千问大语言模型**（商业版、开源版）
- **代码模型**（Qwen-Coder 系列）
- **通义千问 VL**（视觉理解模型）
- **深入研究模型**（Qwen Deep Research）
- **通义千问 Audio**（音频理解和转录）
- **数学模型**（Qwen-Math 系列）

---

## 请求参数详解

### 核心参数

#### model (string, 必选)

模型名称。

**支持的模型类型**：
- 通义千问大语言模型（商业版、开源版）
- 代码模型
- 通义千问VL
- 深入研究模型
- 通义千问Audio
- 数学模型

具体模型名称和计费，请参见[模型列表](https://help.aliyun.com/zh/model-studio/getting-started/models)。

**示例**：
```json
{
  "model": "qwen-plus"
}
```

#### messages (array, 必选)

由历史对话组成的消息列表。

**注意**：通过 HTTP 调用时，请将 `messages` 放入 `input` 对象中。

**结构**：
```json
{
  "input": {
    "messages": [
      {
        "role": "system",
        "content": "You are a helpful assistant."
      },
      {
        "role": "user",
        "content": "你好"
      }
    ]
  }
}
```

---

## 消息类型

### 1. System Message (可选)

模型的目标或角色。如果设置系统消息，请放在 `messages` 列表的第一位。

**属性**：

| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| `role` | string | 是 | 固定为 `"system"` |
| `content` | string 或 array | 是 | 消息内容。仅在调用录音文件识别-通义千问功能时为 array 类型；其他情况为 string 类型 |

**content 为 array 时的属性**（仅录音文件识别）：

| 字段 | 类型 | 说明 |
|------|------|------|
| `text` | string | 指定上下文（Context）。通义千问3 ASR 支持用户在语音识别的同时，提供背景文本、实体词表等参考信息 |

**注意**：
- QwQ 模型不建议设置 System Message
- QVQ 模型设置 System Message 不会生效

**示例**：
```json
{
  "role": "system",
  "content": "You are a helpful assistant."
}
```

### 2. User Message (必选)

用户发送给模型的消息。

**属性**：

| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| `role` | string | 是 | 固定为 `"user"` |
| `content` | string 或 array | 是 | 用户消息的内容。如果只有文本为 string；如果包含图像等多模态数据为 array |
| `cache_control` | object | 否 | 用于开启显式缓存（仅支持显式缓存的模型） |

**content 为 array 时的属性**（多模态输入）：

| 字段 | 类型 | 说明 |
|------|------|------|
| `text` | string | 传入的文本信息 |
| `image` | string | 使用 Qwen-VL 模型进行图片理解时，传入的图片文件。可以为图片的 URL 或本地路径 |
| `enable_rotate` | boolean | 使用通义千问 OCR 模型进行文字提取前对图像进行自动转正。默认值：false |
| `video` | array 或 string | 使用 Qwen-VL 模型或 QVQ 模型进行视频理解时传入的视频文件 |
| `fps` | float | 用于控制抽帧的频率 |
| `audio` | string | 模型为音频理解或语音识别类模型时的音频文件 |
| `min_pixels` | integer | 设定输入图像的最小像素阈值 |
| `max_pixels` | integer | 设定输入图像的最大像素阈值 |

**cache_control 属性**：

| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| `type` | string | 是 | 固定为 `"ephemeral"` |

**示例**：

纯文本：
```json
{
  "role": "user",
  "content": "你是谁？"
}
```

多模态（图像）：
```json
{
  "role": "user",
  "content": [
    {"image": "https://example.com/image.jpg"},
    {"text": "这张图片是什么？"}
  ]
}
```

多模态（视频）：
```json
{
  "role": "user",
  "content": [
    {
      "video": [
        "https://example.com/frame1.jpg",
        "https://example.com/frame2.jpg"
      ],
      "fps": 2.0
    },
    {"text": "描述这个视频"}
  ]
}
```

多模态（音频）：
```json
{
  "role": "user",
  "content": [
    {"audio": "https://example.com/audio.mp3"},
    {"text": "这段音频在说什么？"}
  ]
}
```

### 3. Assistant Message (可选)

模型对用户消息的回复。

**属性**：

| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| `role` | string | 是 | 固定为 `"assistant"` |
| `content` | string | 否 | 消息内容。仅当助手消息中指定 tool_calls 参数时非必选 |
| `partial` | boolean | 否 | 是否开启 Partial Mode（前缀续写） |
| `tool_calls` | array | 否 | 在发起 Function Calling 后，模型回复的要调用的工具和参数 |

**partial 支持的模型**：
- 通义千问 Max 系列：qwen-max、qwen-max-latest、qwen-max-2024-09-19 及之后
- 通义千问 Plus 系列（非思考模式）
- 通义千问 Flash 系列（非思考模式）
- 通义千问 Coder 系列
- 通义千问 VL 系列
- 通义千问 Turbo 系列（非思考模式）
- 通义千问开源系列
- 通义千问 Math 系列

**tool_calls 属性**：

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | string | 本次工具响应的 ID |
| `type` | string | 工具的类型，当前只支持 `"function"` |
| `function` | object | 需要被调用的函数 |
| `function.index` | integer | 工具信息在 tool_calls 列表中的索引 |

**示例**：
```json
{
  "role": "assistant",
  "content": "我是通义千问，由阿里云开发的大语言模型。"
}
```

### 4. Tool Message (可选)

工具的输出信息。

**属性**：

| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| `role` | string | 是 | 固定为 `"tool"` |
| `content` | string | 是 | 工具消息的内容，一般为工具函数的输出 |
| `tool_call_id` | string | 否 | 发起 Function Calling 后返回的 id，用于标记 Tool Message 对应的工具 |

**示例**：
```json
{
  "role": "tool",
  "content": "{\"temperature\": 22, \"weather\": \"晴天\"}",
  "tool_call_id": "call_abc123"
}
```

---

## 请求参数（Parameters）

以下参数通过 HTTP 调用时，请放入 `parameters` 对象中。

### 采样控制参数

#### temperature (float, 可选)

采样温度，控制模型生成文本的多样性。

- **取值范围**：`[0, 2)`
- **默认值**：根据模型不同而不同
  - Qwen3、qwen-max、qwen-plus、qwen-flash 等：0.7
  - QVQ 系列：0.5
  - qwen-vl 系列、qwen-vl-ocr 系列：0.01
  - qwen-math 系列：0
  - QwQ 系列、Qwen3-Thinking：0.6
  - qwen-long 系列：1.0

**说明**：
- temperature 越高，生成的文本更多样
- temperature 越低，生成的文本更确定
- 不建议修改 QVQ 模型的默认 temperature 值

**示例**：
```json
{
  "parameters": {
    "temperature": 0.7
  }
}
```

#### top_p (float, 可选)

核采样的概率阈值，控制模型生成文本的多样性。

- **取值范围**：`(0, 1.0]`
- **默认值**：根据模型不同而不同
  - Qwen3、qwen-max、qwen-plus 等：0.8
  - qwen-vl-max、qwen2-vl-72b-instruct：0.01
  - qwen-vl-plus、qwen-vl-ocr 系列：0.001
  - QVQ 系列、qwen2-audio-instruct：0.5
  - qwen-math 系列：1.0
  - QwQ 系列、Qwen3-Thinking：0.95

**说明**：
- top_p 越高，生成的文本更多样
- top_p 越低，生成的文本更确定
- Java SDK 中为 `topP`
- 不建议修改 QVQ 模型的默认 top_p 值

#### top_k (integer, 可选)

生成过程中采样候选集的大小。

- **取值范围**：≥ 0
- **默认值**：根据模型不同而不同
  - QVQ 系列：10
  - QwQ 系列：40
  - qwen-math、qwen-vl-plus、qwen-vl-ocr 等：1
  - 其余模型：20

**说明**：
- 取值越大，生成的随机性越高
- 取值越小，生成的确定性越高
- 当 top_k 为 None 或大于 100 时，表示不启用 top_k 策略
- Java SDK 中为 `topK`
- 不建议修改 QVQ 模型的默认 top_k 值

#### repetition_penalty (float, 可选)

模型生成时连续序列中的重复度。

- **取值范围**：> 0
- **默认值**：根据模型不同而不同
  - qwen-max、qwen-math、qwen-vl-max、QVQ、QwQ 等：1.0
  - qwen-coder、qwen2.5-1.5b-instruct 等：1.1
  - qwen-vl-plus：1.2
  - 其余模型：1.05

**说明**：
- 提高 repetition_penalty 可以降低模型生成的重复度
- 1.0 表示不做惩罚
- Java SDK 中为 `repetitionPenalty`
- 对于通义千问 OCR 模型，该参数对模型效果影响较大，请勿随意修改
- 不建议修改 QVQ 模型的默认值

#### presence_penalty (float, 可选)

控制模型生成文本时的内容重复度。

- **取值范围**：`[-2.0, 2.0]`
- **默认值**：根据模型不同而不同
  - Qwen3、qwen-max、qwen-vl-max 等：1.5
  - qwen-vl-plus-latest：1.2
  - qwen-vl-plus-2025-01-25：1.0
  - qwen3 思考模式、qwen-plus 思考模式等：0.5
  - 其余均为：0.0

**说明**：
- 正数会减少重复度，负数会增加重复度
- 较高的值适用于要求多样性、趣味性或创造性的场景
- 较低的值适用于要求一致性或专业术语的场景
- Java SDK 不支持设置该参数
- 不建议修改 QVQ 模型的默认值

**示例**：
```json
{
  "parameters": {
    "temperature": 0.7,
    "top_p": 0.8,
    "top_k": 20,
    "repetition_penalty": 1.05,
    "presence_penalty": 1.5
  }
}
```

### 思考模式参数

#### enable_thinking (boolean, 可选)

是否开启思考模式。

- **适用模型**：Qwen3、Qwen3-VL 商业版与开源版、Qwen3-Omni-Flash 模型
- **默认值**：
  - Qwen3 开源版：true
  - Qwen3 商业版：false

**说明**：
- Java SDK 为 `enableThinking`

#### thinking_budget (integer, 可选)

思考过程的最大长度。

- **生效条件**：enable_thinking 为 true 时生效
- **适用模型**：Qwen3 全系、Qwen3-VL 模型
- **默认值**：模型最大思维链长度

### 输出控制参数

#### max_tokens (integer, 可选)

本次请求返回的最大 Token 数。

- **默认值和最大值**：模型的最大输出长度
- **注意**：
  - max_tokens 的设置不会影响大模型的生成过程
  - 如果模型生成的 Token 数超过 max_tokens，本次请求会返回截断后的内容
  - 对于 QwQ、QVQ 与开启思考模式的 Qwen3 模型，max_tokens 会限制回复内容的长度，不限制深度思考内容的长度
- Java SDK 中为 `maxTokens`

#### max_input_tokens (integer, 可选)

允许输入的最大 Token 长度。

- **支持模型**：qwen-plus-0728/latest 模型
- **默认值**：
  - qwen-plus-latest：129,024（后续可能调整至 1,000,000）
  - qwen-plus-2025-07-28：1,000,000

#### seed (integer, 可选)

设置 seed 参数会使文本生成过程更具确定性。

- **取值范围**：0 到 2³¹−1
- **默认值**：
  - qwen-vl-plus-2025-01-02、qwen-vl-max、QVQ 系列：3407
  - qwen-vl-max、qwen2-vl 等部分模型：无默认值
  - 其余模型：1234

**说明**：
- 在每次模型调用时传入相同的 seed 值，并保持其他参数不变，模型将尽可能返回相同的结果

#### stream (boolean, 可选)

是否流式输出回复。

- **参数值**：
  - `false`（默认值）：模型生成完所有内容后一次性返回结果
  - `true`：边生成边输出，即每生成一部分内容就立即输出一个片段（chunk）

**说明**：
- 该参数仅支持 Python SDK
- 通过 Java SDK 实现流式输出请通过 `streamCall` 接口调用
- 通过 HTTP 实现流式输出请在 Header 中指定 `X-DashScope-SSE` 为 `enable`
- Qwen3 商业版（思考模式）、Qwen3 开源版、QwQ、QVQ 只支持流式输出

#### incremental_output (boolean, 可选)

在流式输出模式下是否开启增量输出。

- **默认值**：
  - Qwen3-Max、Qwen3-VL、Qwen3 开源版、QwQ、QVQ：true
  - 其他模型：false

**参数值**：
- `false`：每次输出为当前已经生成的整个序列
- `true`：增量输出，后续输出内容不包含已输出的内容

**说明**：
- Java SDK 中为 `incrementalOutput`
- QwQ 模型与思考模式下的 Qwen3 模型只支持设置为 true
- Qwen3 开源版模型不支持设置为 false
- 推荐优先设置为 true

#### stop (string 或 array, 可选)

使用 stop 参数后，当模型生成的文本即将包含指定的字符串或 token_id 时，将自动停止生成。

**说明**：
- 您可以在 stop 参数中传入敏感词来控制模型的输出
- stop 为 array 类型时，不可以将 token_id 和字符串同时作为元素输入

**示例**：
```json
{
  "parameters": {
    "stop": ["\n\n", "结束"]
  }
}
```

### 输出格式参数

#### result_format (string, 可选)

返回数据的格式。

- **可选值**：`"text"` 或 `"message"`
- **默认值**：
  - Qwen3-Max、Qwen3-VL、QwQ、Qwen3 开源模型、Qwen-Long：`"message"`
  - 其他模型：`"text"`

**说明**：
- 推荐优先设置为 `"message"`，可以更方便地进行多轮对话
- Java SDK 中为 `resultFormat`
- Qwen3-Max、Qwen3-VL、思考模式下的 Qwen3 模型只能设置为 `"message"`

#### response_format (object, 可选)

返回内容的格式。

- **可选值**：`{"type": "text"}` 或 `{"type": "json_object"}`
- **默认值**：`{"type": "text"}`

**说明**：
- 设置为 `{"type": "json_object"}` 时会输出标准格式的 JSON 字符串
- 如果指定该参数为 `{"type": "json_object"}`，您需要在 System Message 或 User Message 中指引模型输出 JSON 格式
- Java SDK 中为 `responseFormat`

**支持的模型**：
- 通义千问 Max 系列
- 通义千问 Plus 系列（非思考模式）
- 通义千问 Flash 系列（非思考模式）
- 通义千问 Coder 系列
- 通义千问 VL 系列
- 通义千问 Turbo 系列（非思考模式）
- Qwen 开源系列

**示例**：
```json
{
  "parameters": {
    "response_format": {"type": "json_object"}
  }
}
```

#### logprobs (boolean, 可选)

是否返回输出 Token 的对数概率。

- **默认值**：false
- **可选值**：
  - `true`：返回
  - `false`：不返回

**说明**：
- 思考阶段生成的内容（reasoning_content）不会返回对数概率
- 支持 qwen-plus、qwen-turbo 系列的快照模型（不包含主线模型）与 Qwen3 开源模型

#### top_logprobs (integer, 可选)

指定在每一步生成时，返回模型最大概率的候选 Token 个数。

- **取值范围**：`[0, 5]`
- **默认值**：0
- **生效条件**：仅当 logprobs 为 true 时生效

#### n (integer, 可选)

生成响应的个数。

- **取值范围**：1-4
- **默认值**：1
- **支持模型**：qwen-plus、Qwen3（非思考模式）、qwen-plus-character 模型
- **限制**：在传入 tools 参数时固定为 1

**说明**：
- 对于需要生成多个响应的场景（如创意写作、广告文案等），可以设置较大的 n 值
- 设置较大的 n 值不会增加输入 Token 消耗，会增加输出 Token 的消耗

### 多模态参数

#### vl_high_resolution_images (boolean, 可选)

是否提高输入图片的默认 Token 上限。

- **默认值**：false
- **适用模型**：Qwen-VL、QVQ 模型

**参数值**：
- `false`（默认值）：使用默认的 Token 处理图像
  - Qwen3-VL、qwen-vl-max-0813 及以后、qwen-vl-plus-0710 及以后：默认 Token 上限为 2560
  - QVQ 及其他 Qwen-VL 模型：默认 Token 上限为 1280
- `true`：输入图片的 Token 上限将提高为 16384

**说明**：
- Java SDK 为 `vlHighResolutionImages`，最低版本为 2.20.8

#### vl_enable_image_hw_output (boolean, 可选)

是否返回图像缩放后的尺寸。

- **默认值**：false
- **适用模型**：Qwen-VL 模型

**说明**：
- 模型会对输入的图像进行缩放处理
- 配置为 true 时会返回图像缩放后的高度和宽度
- 开启流式输出时，该信息在最后一个数据块（chunk）中返回
- Java SDK 中为 `vlEnableImageHwOutput`，最低版本为 2.20.8

### OCR 参数

#### ocr_options (object, 可选)

当您使用通义千问 OCR 模型执行内置任务时需要配置的参数。

**属性**：

| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| `task` | string | 是 | 内置任务的名称 |
| `task_config` | array | 否 | 当 task 为 `"key_information_extraction"` 时使用 |

**task 可选值**：
- `"text_recognition"`：通用文字识别
- `"key_information_extraction"`：信息抽取
- `"document_parsing"`：文档解析
- `"table_parsing"`：表格解析
- `"formula_recognition"`：公式识别
- `"multi_lan"`：多语言识别
- `"advanced_recognition"`：高精识别

**task_config 属性**（信息抽取）：

| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| `result_schema` | object | 是 | 需要模型抽取的字段，可以是任意形式的 JSON 结构，最多可嵌套 3 层 |

**示例**：
```json
{
  "parameters": {
    "ocr_options": {
      "task": "key_information_extraction",
      "task_config": {
        "result_schema": {
          "收件人信息": {
            "收件人姓名": "",
            "收件人电话号码": "",
            "收件人地址": ""
          }
        }
      }
    }
  }
}
```

### ASR 参数

#### asr_options (object, 可选)

该参数仅在调用录音文件识别-通义千问功能时可用，且仅对通义千问3 ASR 模型生效。

**属性**：

| 字段 | 类型 | 可选值 | 默认值 | 说明 |
|------|------|--------|--------|------|
| `language` | string | zh, en, ja, de, ko, ru, fr, pt, ar, it, es | 无 | 若已知音频的语种，可通过该参数指定 |
| `enable_itn` | boolean | true/false | false | 是否启用 ITN（逆文本规范化） |
| `enable_lid` | boolean | true/false | false | 是否在识别结果中返回语种识别信息 |

**说明**：
- `language`：只能指定一个语种；若音频语种不确定或包含多种语种，请勿指定该参数
- `enable_itn`：该功能仅适用于中文和英文音频
- Java SDK 不支持该参数

**示例**：
```json
{
  "parameters": {
    "asr_options": {
      "language": "zh",
      "enable_itn": true,
      "enable_lid": true
    }
  }
}
```

### 工具调用参数

#### tools (array, 可选)

可供模型调用的工具数组，可以包含一个或多个工具对象。

**说明**：
- 一次 Function Calling 流程模型会从中选择其中一个工具
- 开启 parallel_tool_calls 参数可能选择多个工具
- 使用 tools 时需要同时指定 result_format 参数为 `"message"`
- 目前不支持通义千问 VL/Audio，也不建议用于数学和代码模型（Qwen3-Coder 模型除外）

**属性**：

| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| `type` | string | 是 | tools 的类型，当前仅支持 `"function"` |
| `function` | object | 是 | 工具函数定义 |

**function 属性**：

| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| `name` | string | 是 | 工具函数的名称，最大长度为 64 |
| `description` | string | 是 | 工具函数的描述，供模型选择何时以及如何调用 |
| `parameters` | object | 是 | 工具的参数描述，需要是一个合法的 JSON Schema |

**示例**：
```json
{
  "parameters": {
    "tools": [
      {
        "type": "function",
        "function": {
          "name": "get_current_weather",
          "description": "查询指定城市的天气",
          "parameters": {
            "type": "object",
            "properties": {
              "location": {
                "type": "string",
                "description": "城市或县区，比如北京市、杭州市"
              }
            },
            "required": ["location"]
          }
        }
      }
    ]
  }
}
```

#### tool_choice (string 或 object, 可选)

在使用 tools 参数时，用于控制模型调用指定工具。

**可选值**：
- `"none"`：不调用工具（tools 参数为空时的默认值）
- `"auto"`：由模型判断是否调用工具（tools 参数不为空时的默认值）
- object 结构：指定模型调用的工具

**object 结构**：
```json
{
  "type": "function",
  "function": {
    "name": "get_current_time"
  }
}
```

**说明**：
- Java SDK 中为 `toolChoice`

#### parallel_tool_calls (boolean, 可选)

是否开启并行工具调用。

- **默认值**：false
- **参数值**：
  - `true`：开启
  - `false`：不开启

### 搜索参数

#### enable_search (boolean, 可选)

模型在生成文本时是否使用互联网搜索结果进行参考。

- **默认值**：false
- **参数值**：
  - `true`：启用互联网搜索
  - `false`：关闭互联网搜索

**支持的模型**：
- 通义千问 Max：qwen3-max、qwen-max 系列
- 通义千问 Plus：qwen-plus 系列
- 通义千问 Flash：qwen-flash 系列
- 通义千问 Turbo：qwen-turbo 系列
- QwQ：qwq-plus（仅支持流式输出）
- Kimi：Moonshot-Kimi-K2-Instruct

**说明**：
- Java SDK 中为 `enableSearch`
- 启用互联网搜索功能可能会增加 Token 的消耗

#### search_options (object, 可选)

联网搜索的策略。仅当 enable_search 为 true 时生效。

**属性**：

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `enable_source` | boolean | false | 在返回结果中是否展示搜索到的信息 |
| `enable_citation` | boolean | false | 是否开启 [1] 或 [ref_1] 样式的角标标注功能 |
| `citation_format` | string | `"[<number>]"` | 角标样式：`"[<number>]"` 或 `"[ref_<number>]"` |
| `forced_search` | boolean | false | 是否强制开启搜索 |
| `search_strategy` | string | `"turbo"` | 搜索策略：`"turbo"` 或 `"max"` |
| `enable_search_extension` | boolean | false | 是否开启特定领域增强 |
| `prepend_search_result` | boolean | false | 在流式输出时，第一个数据包是否只包含搜索来源信息 |

**说明**：
- Java SDK 中为 `searchOptions`
- `enable_citation`：在 enable_source 为 true 时生效
- `citation_format`：在 enable_citation 为 true 时生效

**示例**：
```json
{
  "parameters": {
    "enable_search": true,
    "search_options": {
      "enable_source": true,
      "enable_citation": true,
      "citation_format": "[<number>]",
      "search_strategy": "turbo"
    }
  }
}
```

### 翻译参数

#### translation_options (object, 可选)

当您使用翻译模型时需要配置的翻译参数。

**属性**：

| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| `source_lang` | string | 是 | 源语言的英文全称，可设置为 `"auto"` 自动判断 |
| `target_lang` | string | 是 | 目标语言的英文全称 |
| `terms` | array | 否 | 术语干预功能的术语数组 |
| `tm_list` | array | 否 | 翻译记忆功能的翻译记忆数组 |
| `domains` | string | 否 | 领域提示功能的领域提示语句（仅支持英文） |

**terms 属性**：

| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| `source` | string | 是 | 源语言的术语 |
| `target` | string | 是 | 目标语言的术语 |

**tm_list 属性**：

| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| `source` | string | 是 | 源语言的语句 |
| `target` | string | 是 | 目标语言的语句 |

**说明**：
- Java SDK 中为 `translationOptions`

### 深入研究参数

#### output_format (string, 可选)

仅当调用通义千问深入研究模型 qwen-deep-research 时，用于规定输出内容格式。

- **默认值**：`"model_detailed_report"`
- **可选值**：
  - `"model_detailed_report"`：详细研究报告，约 6000 字
  - `"model_summary_report"`：摘要研究报告，约 1500-2000 字

### 内容安全参数

#### X-DashScope-DataInspection (string, 可选)

在通义千问 API 的内容安全能力基础上，是否进一步识别输入输出内容的违规信息。

**取值**：
- `'{"input":"cip","output":"cip"}'`：进一步识别
- 不设置该参数：不进一步识别

**说明**：
- 通过 HTTP 调用时请放入请求头
- 通过 Python SDK 调用时请通过 headers 配置
- 不支持通过 Java SDK 设置
- 不适用于 Qwen-VL、Qwen-Audio 系列模型

**示例**：
```bash
--header "X-DashScope-DataInspection: {\"input\": \"cip\", \"output\": \"cip\"}"
```

---

## 响应格式

### 响应对象结构

流式与非流式输出格式一致。

```json
{
  "status_code": 200,
  "request_id": "902fee3b-f7f0-9a8c-96a1-6b4ea25af114",
  "code": "",
  "message": "",
  "output": {
    "text": null,
    "finish_reason": null,
    "choices": [
      {
        "finish_reason": "stop",
        "message": {
          "role": "assistant",
          "content": "我是阿里云开发的一款超大规模语言模型，我叫通义千问。"
        }
      }
    ]
  },
  "usage": {
    "input_tokens": 22,
    "output_tokens": 17,
    "total_tokens": 39
  }
}
```

### 响应字段说明

#### status_code (string)

本次请求的状态码。200 表示请求成功，否则表示请求失败。

**说明**：
- Java SDK 不会返回该参数
- 调用失败会抛出异常，异常信息为 status_code 和 message 的内容

#### request_id (string)

本次调用的唯一标识符。

**说明**：
- Java SDK 返回参数为 `requestId`

#### code (string)

错误码，调用成功时为空值。

**说明**：
- 只有 Python SDK 返回该参数

#### message (string)

错误消息。

#### output (object)

调用结果信息。

**属性**：

##### text (string)

模型生成的回复。当设置输入参数 result_format 为 text 时将回复内容返回到该字段。

##### finish_reason (string)

当设置输入参数 result_format 为 text 时该参数不为空。

**可能的值**：
- `null`：正在生成时
- `"stop"`：因模型输出自然结束，或触发输入参数中的 stop 条件而结束
- `"length"`：因生成长度过长而结束
- `"tool_calls"`：因发生工具调用

##### choices (array)

模型的输出信息。当 result_format 为 message 时返回 choices 参数。

**属性**：

###### finish_reason (string)

**可能的值**：
- `null`：正在生成时
- `"stop"`：因模型输出自然结束，或触发 stop 条件
- `"length"`：因生成长度过长而结束
- `"tool_calls"`：因发生工具调用

###### message (object)

模型输出的消息对象。

**属性**：

| 字段 | 类型 | 说明 |
|------|------|------|
| `role` | string | 输出消息的角色，固定为 `"assistant"` |
| `content` | string 或 array | 输出消息的内容 |
| `reasoning_content` | string | Qwen3、QwQ、QVQ 模型的深度思考内容 |
| `tool_calls` | array | 如果模型需要调用工具，则会生成 tool_calls 参数 |
| `phase` | string | 仅 qwen-deep-research 模型返回，表示研究任务所处的阶段 |
| `extra` | dict | 仅 qwen-deep-research 模型返回，表示研究任务的附加信息 |

**content 为 array 时的属性**（qwen-vl 或 qwen-audio 系列）：

| 字段 | 类型 | 说明 |
|------|------|------|
| `text` | string | 输出消息的内容或语音识别结果 |
| `image_hw` | array | Qwen-VL 系列模型启用 vl_enable_image_hw_output 时返回 |
| `ocr_result` | array | Qwen-OCR 系列模型调用内置任务时的输出结果 |
| `annotations` | array | 录音文件识别-通义千问时的标注信息（如语种） |

**tool_calls 属性**：

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | string | 本次工具响应的 ID |
| `type` | string | 工具类型，固定为 `"function"` |
| `index` | integer | 当前 tool_calls 对象在 tool_calls 数组中的索引 |
| `function` | object | 调用工具的名称和输入参数 |
| `function.name` | string | 调用工具的名称 |
| `function.arguments` | string | 需要输入到工具中的参数，为 JSON 字符串 |

###### logprobs (object)

当前 choices 对象的概率信息。

**属性**：

| 字段 | 类型 | 说明 |
|------|------|------|
| `content` | array | 带有对数概率信息的 Token 数组 |

**content 数组元素属性**：

| 字段 | 类型 | 说明 |
|------|------|------|
| `token` | string | 当前 Token |
| `bytes` | array | 当前 Token 的 UTF‑8 原始字节列表 |
| `logprob` | float | 当前 Token 的对数概率，null 表示概率值极低 |
| `top_logprobs` | array | 当前 Token 位置最可能的若干个 Token 及其对数概率 |

###### search_info (object)

联网搜索到的信息，在设置 search_options 参数后会返回该参数。

**属性**：

| 字段 | 类型 | 说明 |
|------|------|------|
| `search_results` | array | 联网搜索到的结果 |
| `extra_tool_info` | array | 开启 enable_search_extension 参数后返回的领域增强信息 |

#### usage (map)

本次 chat 请求使用的 Token 信息。

**属性**：

| 字段 | 类型 | 说明 |
|------|------|------|
| `input_tokens` | integer | 用户输入内容转换成 Token 后的长度 |
| `output_tokens` | integer | 模型输出内容转换成 Token 后的长度 |
| `total_tokens` | integer | input_tokens 与 output_tokens 之和（纯文本输入时） |
| `image_tokens` | integer | 输入图片内容转换成 Token 后的长度 |
| `video_tokens` | integer | 输入视频内容转换成 Token 后的长度 |
| `audio_tokens` | integer | 输入音频内容转换成 Token 后的长度 |
| `seconds` | integer | 录音文件识别时的音频时长（单位为秒） |
| `input_tokens_details` | object | 输入内容 Token 长度详情 |
| `output_tokens_details` | object | 输出内容 Token 长度详情 |
| `prompt_tokens_details` | object | 输入 Token 的细粒度分类 |
| `cache_creation` | object | 显式缓存创建信息 |

**input_tokens_details 属性**：

| 字段 | 类型 | 说明 |
|------|------|------|
| `text_tokens` | integer | 输入的文本转换为 Token 后的长度 |
| `image_tokens` | integer | 输入的图像转换为 Token 后的长度 |
| `video_tokens` | integer | 输入的视频转换为 Token 后的长度 |

**output_tokens_details 属性**：

| 字段 | 类型 | 说明 |
|------|------|------|
| `text_tokens` | integer | 输出的文本转换为 Token 后的长度 |
| `reasoning_tokens` | integer | Qwen3 模型思考过程转换为 Token 后的长度 |

**prompt_tokens_details 属性**：

| 字段 | 类型 | 说明 |
|------|------|------|
| `cached_tokens` | integer | 命中 Cache 的 Token 数 |

**cache_creation 属性**：

| 字段 | 类型 | 说明 |
|------|------|------|
| `cache_creation_input_tokens` | integer | 用于创建显式缓存的 Token 长度 |
| `cache_type` | string | 使用显式缓存时为 `"ephemeral"` |

---

## 使用示例

### 1. 基本文本对话

**请求示例**：
```bash
curl --location "https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation" \
--header "Authorization: Bearer $DASHSCOPE_API_KEY" \
--header "Content-Type: application/json" \
--data '{
    "model": "qwen-plus",
    "input":{
        "messages":[      
            {
                "role": "system",
                "content": "You are a helpful assistant."
            },
            {
                "role": "user",
                "content": "你是谁？"
            }
        ]
    },
    "parameters": {
        "result_format": "message"
    }
}'
```

**响应示例**：
```json
{
  "status_code": 200,
  "request_id": "902fee3b-f7f0-9a8c-96a1-6b4ea25af114",
  "output": {
    "choices": [
      {
        "finish_reason": "stop",
        "message": {
          "role": "assistant",
          "content": "我是阿里云开发的一款超大规模语言模型，我叫通义千问。"
        }
      }
    ]
  },
  "usage": {
    "input_tokens": 22,
    "output_tokens": 17,
    "total_tokens": 39
  }
}
```

### 2. 流式输出

**请求示例**：
```bash
curl --location "https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation" \
--header "Authorization: Bearer $DASHSCOPE_API_KEY" \
--header "Content-Type: application/json" \
--header "X-DashScope-SSE: enable" \
--data '{
    "model": "qwen-plus",
    "input":{
        "messages":[      
            {
                "role": "system",
                "content": "You are a helpful assistant."
            },
            {
                "role": "user",
                "content": "你是谁？"
            }
        ]
    },
    "parameters": {
        "result_format": "message",
        "incremental_output": true
    }
}'
```

**说明**：
- 流式输出需要在 Header 中添加 `X-DashScope-SSE: enable`
- 推荐设置 `incremental_output` 为 `true` 以获得增量输出

### 3. 图像输入（多模态）

**请求示例**：
```bash
curl --location 'https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation' \
--header "Authorization: Bearer $DASHSCOPE_API_KEY" \
--header 'Content-Type: application/json' \
--data '{
    "model": "qwen-vl-plus",
    "input":{
        "messages":[
            {
                "role": "user",
                "content": [
                    {"image": "https://example.com/dog.jpeg"},
                    {"image": "https://example.com/tiger.png"},
                    {"image": "https://example.com/rabbit.png"},
                    {"text": "这些是什么?"}
                ]
            }
        ]
    }
}'
```

**说明**：
- 多模态请求使用 `/multimodal-generation/generation` 端点
- 图像可以是 URL 或本地路径
- 可以同时传入多张图片

### 4. 视频输入

**方式 1：传入图像列表**：
```bash
curl -X POST https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation \
-H "Authorization: Bearer $DASHSCOPE_API_KEY" \
-H 'Content-Type: application/json' \
-d '{
  "model": "qwen-vl-max-latest",
  "input": {
    "messages": [
      {
        "role": "user",
        "content": [
          {
            "video": [
              "https://example.com/frame1.jpg",
              "https://example.com/frame2.jpg",
              "https://example.com/frame3.jpg",
              "https://example.com/frame4.jpg"
            ],
            "fps": 2.0
          },
          {
            "text": "描述这个视频的具体过程"
          }
        ]
      }
    ]
  }
}'
```

**方式 2：直接传入视频文件**：
```bash
curl -X POST https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation \
-H "Authorization: Bearer $DASHSCOPE_API_KEY" \
-H 'Content-Type: application/json' \
-d '{
  "model": "qwen-vl-max-latest",
  "input": {
    "messages": [
      {
        "role": "user",
        "content": [
          {
            "video": "https://example.com/video.mp4",
            "fps": 2.0
          },
          {
            "text": "这个视频讲了什么？"
          }
        ]
      }
    ]
  }
}'
```

### 5. 音频输入

**音频理解示例**：
```bash
curl --location 'https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation' \
--header "Authorization: Bearer $DASHSCOPE_API_KEY" \
--header 'Content-Type: application/json' \
--data '{
    "model": "qwen2-audio-instruct",
    "input":{
        "messages":[
            {
                "role": "system",
                "content": [
                    {"text": "You are a helpful assistant."}
                ]
            },
            {
                "role": "user",
                "content": [
                    {"audio": "https://example.com/audio.mp3"},
                    {"text": "这段音频在说什么?"}
                ]
            }
        ]
    }
}'
```

**语音识别示例**：
```bash
curl --location 'https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation' \
--header "Authorization: Bearer $DASHSCOPE_API_KEY" \
--header 'Content-Type: application/json' \
--data '{
    "model": "qwen3-asr-flash",
    "input": {
        "messages": [
            {
                "role": "system",
                "content": [
                    {"text": ""}
                ]
            },
            {
                "role": "user",
                "content": [
                    {"audio": "https://example.com/audio.mp3"}
                ]
            }
        ]
    },
    "parameters": {
        "asr_options": {
            "enable_lid": true,
            "enable_itn": false,
            "language": "zh"
        }
    }
}'
```

### 6. 联网搜索

**请求示例**：
```bash
curl -X POST https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation \
-H "Authorization: Bearer $DASHSCOPE_API_KEY" \
-H "Content-Type: application/json" \
-d '{
    "model": "qwen-plus",
    "input":{
        "messages":[      
            {
                "role": "system",
                "content": "You are a helpful assistant."
            },
            {
                "role": "user",
                "content": "明天杭州天气如何？"
            }
        ]
    },
    "parameters": {
        "enable_search": true,
        "search_options": {
            "enable_source": true,
            "enable_citation": true,
            "search_strategy": "turbo"
        },
        "result_format": "message"
    }
}'
```

**说明**：
- 启用 `enable_search` 开启联网搜索
- 通过 `search_options` 配置搜索策略和输出格式

### 7. 工具调用（Function Calling）

**步骤 1：发起工具调用**：
```bash
curl --location "https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation" \
--header "Authorization: Bearer $DASHSCOPE_API_KEY" \
--header "Content-Type: application/json" \
--data '{
    "model": "qwen-plus",
    "input": {
        "messages": [{
            "role": "user",
            "content": "杭州天气怎么样"
        }]
    },
    "parameters": {
        "result_format": "message",
        "tools": [{
            "type": "function",
            "function": {
                "name": "get_current_time",
                "description": "当你想知道现在的时间时非常有用。",
                "parameters": {}
            }
        },{
            "type": "function",
            "function": {
                "name": "get_current_weather",
                "description": "当你想查询指定城市的天气时非常有用。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "城市或县区，比如北京市、杭州市、余杭区等。"
                        }
                    },
                    "required": ["location"]
                }
            }
        }]
    }
}'
```

**响应示例**：
```json
{
  "output": {
    "choices": [
      {
        "finish_reason": "tool_calls",
        "message": {
          "role": "assistant",
          "content": "",
          "tool_calls": [
            {
              "id": "call_abc123",
              "type": "function",
              "function": {
                "name": "get_current_weather",
                "arguments": "{\"location\":\"杭州市\"}"
              }
            }
          ]
        }
      }
    ]
  }
}
```

**步骤 2：提交工具执行结果**：
```bash
curl --location "https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation" \
--header "Authorization: Bearer $DASHSCOPE_API_KEY" \
--header "Content-Type: application/json" \
--data '{
    "model": "qwen-plus",
    "input": {
        "messages": [
            {
                "role": "user",
                "content": "杭州天气怎么样"
            },
            {
                "role": "assistant",
                "content": "",
                "tool_calls": [
                    {
                        "id": "call_abc123",
                        "type": "function",
                        "function": {
                            "name": "get_current_weather",
                            "arguments": "{\"location\":\"杭州市\"}"
                        }
                    }
                ]
            },
            {
                "role": "tool",
                "content": "{\"temperature\": 22, \"weather\": \"晴天\"}",
                "tool_call_id": "call_abc123"
            }
        ]
    },
    "parameters": {
        "result_format": "message",
        "tools": [...]
    }
}'
```

### 8. OCR 文字提取

**信息抽取示例**：
```bash
curl --location 'https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation' \
--header "Authorization: Bearer $DASHSCOPE_API_KEY" \
--header 'Content-Type: application/json' \
--data '{
  "model": "qwen-vl-ocr-latest",
  "input": {
    "messages": [
      {
        "role": "user",
        "content": [
          {
            "image": "https://example.com/invoice.jpg",
            "min_pixels": 3136,
            "max_pixels": 6422528,
            "enable_rotate": true
          },
          {
            "text": "假设你是一名信息提取专家。现在给你一个JSON模式，用图像中的信息填充该模式的值部分。请注意，如果值是一个列表，模式将为每个元素提供一个模板。当图像中有多个列表元素时，将使用此模板。最后，只需要输出合法的JSON。所见即所得，并且输出语言需要与图像保持一致。模糊或者强光遮挡的单个文字可以用英文问号?代替。如果没有对应的值则用null填充。不需要解释。请按要求输出结果。输入的JSON模式内容如下: {result_schema}。"
          }
        ]
      }
    ]
  },
  "parameters": {
    "ocr_options": {
      "task": "key_information_extraction",
      "task_config": {
        "result_schema": {
          "销售方名称": "",
          "购买方名称": "",
          "不含税价": "",
          "组织机构代码": "",
          "发票代码": ""
        }
      }
    }
  }
}'
```

### 9. 深入研究（Deep Research）

**第一步：模型反问确认**：
```bash
curl --location 'https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation' \
--header 'X-DashScope-SSE: enable' \
--header "Authorization: Bearer $DASHSCOPE_API_KEY" \
--header 'Content-Type: application/json' \
--data '{
    "input": {
        "messages": [
            {
                "content": "研究一下人工智能在教育中的应用", 
                "role": "user"
            }
        ]
    },
    "model": "qwen-deep-research"
}'
```

**第二步：深入研究**：
```bash
curl --location 'https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation' \
--header 'X-DashScope-SSE: enable' \
--header "Authorization: Bearer $DASHSCOPE_API_KEY" \
--header 'Content-Type: application/json' \
--data '{
    "input": {
        "messages": [
            {
                "content": "研究一下人工智能在教育中的应用", 
                "role": "user"
            },
            {
                "content": "请告诉我您希望重点研究人工智能在教育中的哪些具体应用场景？", 
                "role": "assistant"
            },
            {
                "content": "我主要关注个性化学习方面", 
                "role": "user"
            }
        ]
    },
    "model": "qwen-deep-research",
    "parameters": {
        "output_format": "model_detailed_report"
    }
}'
```

---

## 特殊功能

### 1. 思考模式

Qwen3 系列模型支持思考模式，可以输出深度思考内容。

**启用思考模式**：
```json
{
  "parameters": {
    "enable_thinking": true,
    "thinking_budget": 1000
  }
}
```

**响应中的思考内容**：
```json
{
  "output": {
    "choices": [
      {
        "message": {
          "role": "assistant",
          "content": "最终回答内容",
          "reasoning_content": "深度思考过程..."
        }
      }
    ]
  }
}
```

### 2. 结构化输出

强制模型输出 JSON 格式。

**请求示例**：
```json
{
  "input": {
    "messages": [
      {
        "role": "system",
        "content": "请按照 JSON 格式输出。"
      },
      {
        "role": "user",
        "content": "列出三个水果的名称和颜色"
      }
    ]
  },
  "parameters": {
    "response_format": {"type": "json_object"}
  }
}
```

### 3. 前缀续写（Partial Mode）

让模型从指定的前缀开始继续生成。

**请求示例**：
```json
{
  "input": {
    "messages": [
      {
        "role": "user",
        "content": "写一首关于春天的诗"
      },
      {
        "role": "assistant",
        "content": "春风拂面",
        "partial": true
      }
    ]
  }
}
```

### 4. 上下文缓存

减少重复输入的 Token 消耗。

**显式缓存示例**：
```json
{
  "input": {
    "messages": [
      {
        "role": "user",
        "content": "这是一个很长的上下文...",
        "cache_control": {
          "type": "ephemeral"
        }
      }
    ]
  }
}
```

**响应中的缓存信息**：
```json
{
  "usage": {
    "prompt_tokens_details": {
      "cached_tokens": 1000
    },
    "cache_creation": {
      "cache_creation_input_tokens": 2000,
      "cache_type": "ephemeral"
    }
  }
}
```

### 5. 并行工具调用

允许模型在一次响应中调用多个工具。

**请求示例**：
```json
{
  "parameters": {
    "tools": [...],
    "parallel_tool_calls": true
  }
}
```

---

## 错误处理

### 常见错误码

| 错误码 | 说明 | 处理方法 |
|--------|------|----------|
| 400 | 请求参数错误 | 检查请求参数是否符合规范 |
| 401 | 认证失败 | 检查 API 密钥是否正确 |
| 403 | 无权限访问 | 确认账户是否有访问该模型的权限 |
| 429 | 请求频率超限 | 降低请求频率或升级配额 |
| 500 | 服务器内部错误 | 稍后重试 |

### 错误响应示例

```json
{
  "status_code": 400,
  "code": "InvalidParameter",
  "message": "Invalid parameter: model",
  "request_id": "xxx-xxx-xxx"
}
```

---

## 最佳实践

### 1. 参数调优建议

| 场景 | temperature | top_p | 说明 |
|------|-------------|-------|------|
| 创意写作 | 0.8-1.0 | 0.9-0.95 | 需要多样性和创造性 |
| 技术文档 | 0.3-0.5 | 0.7-0.8 | 需要准确性和一致性 |
| 代码生成 | 0.2-0.4 | 0.7-0.8 | 需要确定性 |
| 聊天对话 | 0.7-0.9 | 0.8-0.9 | 平衡多样性和相关性 |

### 2. Token 优化

- **使用上下文缓存**：对于重复的长上下文，使用 cache_control 参数
- **控制输出长度**：合理设置 max_tokens 参数
- **使用 stop 参数**：当生成达到期望状态时提前停止

### 3. 性能优化

- **使用流式输出**：对于长文本生成，使用流式输出提升用户体验
- **设置 incremental_output**：在流式模式下设置为 true，减少重复内容
- **选择合适的模型**：根据任务复杂度选择合适的模型（Plus vs Max vs Flash）

### 4. 安全建议

- **不要在日志中记录 API 密钥**：确保 API 密钥安全
- **使用内容审核**：启用 X-DashScope-DataInspection 参数
- **设置 stop 参数**：防止模型生成不适当的内容

---

## 附录

### 支持的语言列表（ASR）

| 语言 | 代码 |
|------|------|
| 中文 | zh |
| 英文 | en |
| 日语 | ja |
| 德语 | de |
| 韩语 | ko |
| 俄语 | ru |
| 法语 | fr |
| 葡萄牙语 | pt |
| 阿拉伯语 | ar |
| 意大利语 | it |
| 西班牙语 | es |

### 模型系列对比

| 系列 | 特点 | 适用场景 |
|------|------|----------|
| Qwen-Max | 最强性能 | 复杂推理、专业领域 |
| Qwen-Plus | 平衡性能 | 日常对话、通用任务 |
| Qwen-Flash | 快速响应 | 简单任务、高并发场景 |
| Qwen-Turbo | 经济实惠 | 成本敏感场景 |
| Qwen-VL | 视觉理解 | 图像、视频理解 |
| Qwen-Audio | 音频理解 | 语音识别、音频分析 |
| Qwen-Coder | 代码生成 | 编程辅助、代码补全 |
| Qwen-Math | 数学解题 | 数学问题求解 |
| QwQ | 深度推理 | 复杂推理任务 |

---

## 参考资源

- [模型列表和定价](https://help.aliyun.com/zh/model-studio/getting-started/models)
- [API 文档](https://help.aliyun.com/zh/model-studio/developer-reference/api-details)
- [SDK 下载](https://help.aliyun.com/zh/model-studio/developer-reference/sdk-overview)
- [示例代码](https://github.com/aliyun/alibabacloud-bailian-demo)

---

**版本**: 1.0  
**最后更新**: 2025-01-09  
**维护者**: 通义千问团队

