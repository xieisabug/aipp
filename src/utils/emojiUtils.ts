import emojiConfig from '@/data/emoji-config.json';

export interface EmojiCategory {
  name: string;
  emojis: string[];
}

export interface EmojiData {
  categories: Record<string, EmojiCategory>;
  defaultCategory: string;
}


/**
 * 获取emoji配置数据
 */
export function getEmojiData(): EmojiData {
  return emojiConfig as EmojiData;
}

/**
 * 获取所有emoji分类
 */
export function getEmojiCategories(): Record<string, EmojiCategory> {
  return emojiConfig.categories;
}

/**
 * 根据分类名获取emoji列表
 */
export function getEmojisByCategory(categoryKey: string): string[] {
  const category = (emojiConfig.categories as Record<string, EmojiCategory>)[categoryKey];
  return category ? category.emojis : [];
}

/**
 * 获取所有emoji的扁平化列表
 */
export function getAllEmojis(): string[] {
  const categories = getEmojiCategories();
  return Object.values(categories).flatMap(category => category.emojis);
}


/**
 * 将文件转换为Base64
 */
export function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      if (reader.result) {
        resolve(reader.result as string);
      } else {
        reject(new Error('Failed to read file'));
      }
    };
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(file);
  });
}

/**
 * 验证图片文件
 */
export function validateImageFile(file: File): { valid: boolean; error?: string } {
  // 检查文件类型
  const allowedTypes = ['image/png', 'image/jpeg', 'image/jpg', 'image/gif', 'image/svg+xml', 'image/webp'];
  if (!allowedTypes.includes(file.type)) {
    return {
      valid: false,
      error: '不支持的文件格式。请选择 PNG、JPG、GIF、SVG 或 WebP 格式的图片。'
    };
  }

  // 检查文件大小 (最大 5MB)
  const maxSize = 5 * 1024 * 1024; // 5MB
  if (file.size > maxSize) {
    return {
      valid: false,
      error: '文件太大。请选择小于 5MB 的图片。'
    };
  }

  return { valid: true };
}

/**
 * 压缩图片到指定尺寸
 */
export function resizeImage(file: File, maxWidth = 64, maxHeight = 64, quality = 0.8): Promise<string> {
  return new Promise((resolve, reject) => {
    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d');
    const img = new Image();

    img.onload = () => {
      // 计算新的尺寸，保持宽高比
      let { width, height } = img;
      if (width > height) {
        if (width > maxWidth) {
          height = (height * maxWidth) / width;
          width = maxWidth;
        }
      } else {
        if (height > maxHeight) {
          width = (width * maxHeight) / height;
          height = maxHeight;
        }
      }

      canvas.width = width;
      canvas.height = height;

      if (ctx) {
        ctx.drawImage(img, 0, 0, width, height);
        const dataUrl = canvas.toDataURL('image/png', quality);
        resolve(dataUrl);
      } else {
        reject(new Error('Canvas context not available'));
      }
    };

    img.onerror = () => reject(new Error('Failed to load image'));
    img.src = URL.createObjectURL(file);
  });
}

/**
 * 检查值是否是Base64图片
 */
export function isBase64Image(value: string): boolean {
  return typeof value === 'string' && value.startsWith('data:image/');
}

/**
 * 获取默认图标
 */
export function getDefaultIcon(): string {
  return '🎨';
}

/**
 * 格式化图标显示
 */
export function formatIconDisplay(icon: string): { display: string; isImage: boolean } {
  if (isBase64Image(icon)) {
    return {
      display: icon,
      isImage: true
    };
  }
  return {
    display: icon,
    isImage: false
  };
}

