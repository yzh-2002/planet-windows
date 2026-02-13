# 图标迁移指南：macOS → Windows (Tauri)

> 将 Planet macOS 项目的图标资源迁移到 Tauri Windows 应用

---

## 目录

1. [图标格式要求](#1-图标格式要求)
2. [准备图标资源](#2-准备图标资源)
3. [转换 PNG 到 ICO](#3-转换-png-到-ico)
4. [配置 Tauri 图标](#4-配置-tauri-图标)
5. [验证图标](#5-验证图标)

---

## 1. 图标格式要求

### macOS vs Windows

| 平台 | 格式 | 尺寸要求 |
|------|------|----------|
| **macOS** | `.icns` | 多尺寸 PNG 打包 |
| **Windows** | `.ico` | 多尺寸 PNG 打包（16x16, 32x32, 48x48, 64x64, 128x128, 256x256） |
| **Tauri** | `.png` + `.ico` | 需要多个 PNG 尺寸 + 一个 ICO 文件 |

### Tauri 图标要求

Tauri 需要以下图标文件：

```
src-tauri/icons/
├── 32x32.png          # 32x32 PNG
├── 128x128.png        # 128x128 PNG
├── 128x128@2x.png     # 256x256 PNG (macOS retina)
├── icon.icns          # macOS 图标包
└── icon.ico           # Windows 图标包
```

---

## 2. 准备图标资源

### 2.1 复制原始图标文件

原项目的图标位于：
```
Planet/Assets.xcassets/AppIcon.appiconset/
├── Planetable Lite 16.png
├── Planetable Lite 32.png
├── Planetable Lite 64.png
├── Planetable Lite 128.png
├── Planetable Lite 256.png
├── Planetable Lite 512.png
└── Planetable Lite 1024.png
```

### 2.2 创建 Tauri 图标目录

```bash
cd planet-desktop/src-tauri
mkdir -p icons
```

### 2.3 复制并重命名图标文件

```bash
# 从原项目复制图标到 Tauri 项目
cp ../Planet/Assets.xcassets/AppIcon.appiconset/"Planetable Lite 32.png" icons/32x32.png
cp ../Planet/Assets.xcassets/AppIcon.appiconset/"Planetable Lite 128.png" icons/128x128.png
cp ../Planet/Assets.xcassets/AppIcon.appiconset/"Planetable Lite 256.png" icons/128x128@2x.png
cp ../Planet/Assets.xcassets/AppIcon.appiconset/"Planetable Lite 512.png" icons/icon.png  # 可选，作为默认图标
```

---

## 3. 转换 PNG 到 ICO

Windows 需要 `.ico` 格式，包含多个尺寸。有几种方法：

### 方法 1：使用在线工具（最简单）

1. 访问 https://convertio.co/png-ico/ 或 https://icoconvert.com/
2. 上传 `Planetable Lite 256.png` 或 `Planetable Lite 512.png`
3. 选择多个尺寸（16x16, 32x32, 48x48, 64x64, 128x128, 256x256）
4. 下载生成的 `icon.ico`
5. 保存到 `src-tauri/icons/icon.ico`

### 方法 2：使用 ImageMagick（命令行）

**macOS:**
```bash
# 安装 ImageMagick
brew install imagemagick

# 转换单个 PNG 到 ICO（包含多个尺寸）
convert Planet/Assets.xcassets/AppIcon.appiconset/"Planetable Lite 256.png" \
  \( -clone 0 -resize 16x16 \) \
  \( -clone 0 -resize 32x32 \) \
  \( -clone 0 -resize 48x48 \) \
  \( -clone 0 -resize 64x64 \) \
  \( -clone 0 -resize 128x128 \) \
  \( -clone 0 -resize 256x256 \) \
  -delete 0 \
  planet-desktop/src-tauri/icons/icon.ico
```

**Windows (PowerShell):**
```powershell
# 安装 ImageMagick: https://imagemagick.org/script/download.php

# 转换
magick convert "Planet\Assets.xcassets\AppIcon.appiconset\Planetable Lite 256.png" `
  ( -clone 0 -resize 16x16 ) `
  ( -clone 0 -resize 32x32 ) `
  ( -clone 0 -resize 48x48 ) `
  ( -clone 0 -resize 64x64 ) `
  ( -clone 0 -resize 128x128 ) `
  ( -clone 0 -resize 256x256 ) `
  -delete 0 `
  planet-desktop\src-tauri\icons\icon.ico
```

### 方法 3：使用 Python 脚本（跨平台）

创建 `scripts/convert_icon.py`:

```python
#!/usr/bin/env python3
"""
将 PNG 图标转换为 ICO 格式（包含多个尺寸）
需要安装: pip install Pillow
"""

from PIL import Image
import sys
import os

def create_ico_from_png(png_path, ico_path):
    """从 PNG 创建包含多个尺寸的 ICO 文件"""
    # 打开原始图片
    img = Image.open(png_path)
    
    # ICO 需要的尺寸列表
    sizes = [(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    
    # 创建不同尺寸的图片
    images = []
    for size in sizes:
        resized = img.resize(size, Image.Resampling.LANCZOS)
        images.append(resized)
    
    # 保存为 ICO（ICO 格式支持多尺寸）
    images[0].save(ico_path, format='ICO', sizes=[(img.width, img.height) for img in images])
    print(f"✅ Created {ico_path} with sizes: {[f'{s[0]}x{s[1]}' for s in sizes]}")

if __name__ == "__main__":
    # 默认路径
    source_png = "../Planet/Assets.xcassets/AppIcon.appiconset/Planetable Lite 256.png"
    target_ico = "src-tauri/icons/icon.ico"
    
    if len(sys.argv) > 1:
        source_png = sys.argv[1]
    if len(sys.argv) > 2:
        target_ico = sys.argv[2]
    
    if not os.path.exists(source_png):
        print(f"❌ Source file not found: {source_png}")
        sys.exit(1)
    
    os.makedirs(os.path.dirname(target_ico), exist_ok=True)
    create_ico_from_png(source_png, target_ico)
```

运行：
```bash
pip install Pillow
python scripts/convert_icon.py
```

### 方法 4：使用 Node.js 脚本

创建 `scripts/convert-icon.js`:

```javascript
const sharp = require('sharp');
const fs = require('fs');
const path = require('path');

async function createIco() {
  const sizes = [16, 32, 48, 64, 128, 256];
  const sourcePng = '../Planet/Assets.xcassets/AppIcon.appiconset/Planetable Lite 256.png';
  const outputIco = 'src-tauri/icons/icon.ico';
  
  // 注意：sharp 不能直接创建 ICO，需要使用其他工具
  // 这里先创建各个尺寸的 PNG，然后用工具合并
  console.log('Creating ICO requires additional tools like imagemagick');
}

createIco();
```

---

## 4. 创建 macOS ICNS 文件（可选）

如果你也想支持 macOS，需要创建 `.icns` 文件：

### 方法 1：使用 iconutil（macOS 自带）

```bash
# 创建 iconset 目录
mkdir -p icon.iconset

# 复制各个尺寸的 PNG
cp "Planet/Assets.xcassets/AppIcon.appiconset/Planetable Lite 16.png" icon.iconset/icon_16x16.png
cp "Planet/Assets.xcassets/AppIcon.appiconset/Planetable Lite 32.png" icon.iconset/icon_16x16@2x.png
cp "Planet/Assets.xcassets/AppIcon.appiconset/Planetable Lite 32.png" icon.iconset/icon_32x32.png
cp "Planet/Assets.xcassets/AppIcon.appiconset/Planetable Lite 64.png" icon.iconset/icon_32x32@2x.png
cp "Planet/Assets.xcassets/AppIcon.appiconset/Planetable Lite 128.png" icon.iconset/icon_128x128.png
cp "Planet/Assets.xcassets/AppIcon.appiconset/Planetable Lite 256.png" icon.iconset/icon_128x128@2x.png
cp "Planet/Assets.xcassets/AppIcon.appiconset/Planetable Lite 256.png" icon.iconset/icon_256x256.png
cp "Planet/Assets.xcassets/AppIcon.appiconset/Planetable Lite 512.png" icon.iconset/icon_256x256@2x.png
cp "Planet/Assets.xcassets/AppIcon.appiconset/Planetable Lite 512.png" icon.iconset/icon_512x512.png
cp "Planet/Assets.xcassets/AppIcon.appiconset/Planetable Lite 1024.png" icon.iconset/icon_512x512@2x.png

# 转换为 ICNS
iconutil -c icns icon.iconset -o src-tauri/icons/icon.icns

# 清理临时目录
rm -rf icon.iconset
```

---

## 5. 配置 Tauri 图标

### 5.1 更新 `tauri.conf.json`

确保 `bundle.icon` 配置正确：

```json
{
  "tauri": {
    "bundle": {
      "icon": [
        "icons/32x32.png",
        "icons/128x128.png",
        "icons/128x128@2x.png",
        "icons/icon.icns",
        "icons/icon.ico"
      ]
    }
  }
}
```

### 5.2 验证图标文件存在

```bash
cd src-tauri
ls -la icons/
```

应该看到：
```
icons/
├── 32x32.png
├── 128x128.png
├── 128x128@2x.png
├── icon.icns      # macOS
└── icon.ico       # Windows
```

---

## 6. 自动化脚本

创建一个完整的迁移脚本 `scripts/setup-icons.sh`:

```bash
#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TAURI_ICONS_DIR="$PROJECT_ROOT/src-tauri/icons"
SOURCE_ICONS_DIR="$PROJECT_ROOT/Planet/Assets.xcassets/AppIcon.appiconset"

echo "🎨 Setting up Tauri icons..."

# 创建图标目录
mkdir -p "$TAURI_ICONS_DIR"

# 复制 PNG 文件
echo "📋 Copying PNG files..."
cp "$SOURCE_ICONS_DIR/Planetable Lite 32.png" "$TAURI_ICONS_DIR/32x32.png"
cp "$SOURCE_ICONS_DIR/Planetable Lite 128.png" "$TAURI_ICONS_DIR/128x128.png"
cp "$SOURCE_ICONS_DIR/Planetable Lite 256.png" "$TAURI_ICONS_DIR/128x128@2x.png"

# 检查 ImageMagick 是否安装
if command -v convert &> /dev/null; then
    echo "🔄 Converting to ICO using ImageMagick..."
    convert "$SOURCE_ICONS_DIR/Planetable Lite 256.png" \
      \( -clone 0 -resize 16x16 \) \
      \( -clone 0 -resize 32x32 \) \
      \( -clone 0 -resize 48x48 \) \
      \( -clone 0 -resize 64x64 \) \
      \( -clone 0 -resize 128x128 \) \
      \( -clone 0 -resize 256x256 \) \
      -delete 0 \
      "$TAURI_ICONS_DIR/icon.ico"
    echo "✅ ICO file created"
else
    echo "⚠️  ImageMagick not found. Please install it or convert manually:"
    echo "   brew install imagemagick  # macOS"
    echo "   Or use online tool: https://convertio.co/png-ico/"
fi

# macOS ICNS (仅 macOS)
if [[ "$OSTYPE" == "darwin"* ]]; then
    if command -v iconutil &> /dev/null; then
        echo "🔄 Creating ICNS for macOS..."
        TEMP_ICONSET=$(mktemp -d)
        
        cp "$SOURCE_ICONS_DIR/Planetable Lite 16.png" "$TEMP_ICONSET/icon_16x16.png"
        cp "$SOURCE_ICONS_DIR/Planetable Lite 32.png" "$TEMP_ICONSET/icon_16x16@2x.png"
        cp "$SOURCE_ICONS_DIR/Planetable Lite 32.png" "$TEMP_ICONSET/icon_32x32.png"
        cp "$SOURCE_ICONS_DIR/Planetable Lite 64.png" "$TEMP_ICONSET/icon_32x32@2x.png"
        cp "$SOURCE_ICONS_DIR/Planetable Lite 128.png" "$TEMP_ICONSET/icon_128x128.png"
        cp "$SOURCE_ICONS_DIR/Planetable Lite 256.png" "$TEMP_ICONSET/icon_128x128@2x.png"
        cp "$SOURCE_ICONS_DIR/Planetable Lite 256.png" "$TEMP_ICONSET/icon_256x256.png"
        cp "$SOURCE_ICONS_DIR/Planetable Lite 512.png" "$TEMP_ICONSET/icon_256x256@2x.png"
        cp "$SOURCE_ICONS_DIR/Planetable Lite 512.png" "$TEMP_ICONSET/icon_512x512.png"
        cp "$SOURCE_ICONS_DIR/Planetable Lite 1024.png" "$TEMP_ICONSET/icon_512x512@2x.png"
        
        iconutil -c icns "$TEMP_ICONSET" -o "$TAURI_ICONS_DIR/icon.icns"
        rm -rf "$TEMP_ICONSET"
        echo "✅ ICNS file created"
    fi
fi

echo "✨ Icon setup complete!"
echo ""
echo "📁 Icons are in: $TAURI_ICONS_DIR"
ls -lh "$TAURI_ICONS_DIR"
```

运行：
```bash
chmod +x scripts/setup-icons.sh
./scripts/setup-icons.sh
```

---

## 7. 验证图标

### 7.1 检查文件

```bash
cd src-tauri/icons
ls -lh
```

应该看到所有图标文件。

### 7.2 测试构建

```bash
cd src-tauri
cargo check
```

### 7.3 构建应用（测试图标是否嵌入）

```bash
pnpm tauri build
```

构建完成后，检查生成的安装包：
- **Windows**: `.msi` 文件应该显示正确的图标
- **macOS**: `.dmg` 文件应该显示正确的图标

---

## 8. 常见问题

### Q1: ICO 文件显示不正确

**原因**: ICO 文件可能只包含单个尺寸  
**解决**: 确保 ICO 文件包含多个尺寸（16, 32, 48, 64, 128, 256）

### Q2: macOS 图标不显示

**原因**: ICNS 文件格式不正确  
**解决**: 使用 `iconutil` 工具重新生成

### Q3: 图标模糊

**原因**: 使用了低分辨率图片  
**解决**: 使用 256x256 或 512x512 的源图片

### Q4: Tauri 找不到图标文件

**原因**: 路径配置错误  
**解决**: 检查 `tauri.conf.json` 中的路径是否相对于 `src-tauri` 目录

---

## 9. 推荐工具

| 工具 | 用途 | 平台 |
|------|------|------|
| **ImageMagick** | PNG → ICO 转换 | 全平台 |
| **iconutil** | PNG → ICNS 转换 | macOS |
| **GIMP** | 图像编辑和转换 | 全平台 |
| **Online ICO Converter** | 在线转换 | 浏览器 |

---

## 10. 快速开始（最简单方法）

如果你只想快速开始，使用在线工具：

1. **复制源图标**:
   ```bash
   cp Planet/Assets.xcassets/AppIcon.appiconset/"Planetable Lite 256.png" planet-desktop/src-tauri/icons/32x32.png
   cp Planet/Assets.xcassets/AppIcon.appiconset/"Planetable Lite 256.png" planet-desktop/src-tauri/icons/128x128.png
   cp Planet/Assets.xcassets/AppIcon.appiconset/"Planetable Lite 256.png" planet-desktop/src-tauri/icons/128x128@2x.png
   ```

2. **在线转换 ICO**:
   - 访问 https://convertio.co/png-ico/
   - 上传 `Planetable Lite 256.png`
   - 下载 `icon.ico`
   - 保存到 `src-tauri/icons/icon.ico`

3. **完成！** ✅

---

## 总结

图标迁移步骤：
1. ✅ 复制 PNG 文件到 `src-tauri/icons/`
2. ✅ 转换 PNG 到 ICO（Windows）
3. ✅ 转换 PNG 到 ICNS（macOS，可选）
4. ✅ 配置 `tauri.conf.json`
5. ✅ 验证构建

完成这些步骤后，你的 Tauri 应用就会使用 Planet 的图标了！🎨
