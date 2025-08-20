/**
 * 闪亮边框的统一配置
 * 使用 Tailwind CSS 变量和主题系统
 */

// 闪亮边框颜色配置
export const SHINE_COLORS = {
    // 使用 Tailwind 颜色变量（推荐）
    default: ["var(--shine-primary)", "var(--shine-secondary)", "var(--shine-tertiary)"] as string[],

    // 兼容旧版本的直接颜色值
    legacy: ["#A07CFE", "#FE8FB5", "#FFBE7B"] as string[],
} as const;

// 闪亮边框的其他配置参数
export const SHINE_CONFIG = {
    borderWidth: 2,
    duration: 8,
    animationClass: "motion-safe:animate-shine",
} as const;

// 完整的闪亮边框配置对象
export const DEFAULT_SHINE_BORDER_CONFIG = {
    shineColor: SHINE_COLORS.default,
    borderWidth: SHINE_CONFIG.borderWidth,
    duration: SHINE_CONFIG.duration,
} as const;

// 类型定义
export type ShineColorType = keyof typeof SHINE_COLORS;

export interface ShineBorderConfig {
    shineColor: string | string[];
    borderWidth: number;
    duration: number;
}

/**
 * 获取闪亮边框配置的辅助函数
 * @param colorType 颜色类型，default 使用 CSS 变量，legacy 使用直接颜色值
 * @returns 完整的闪亮边框配置
 */
export function getShineConfig(colorType: ShineColorType = "default"): ShineBorderConfig {
    return {
        shineColor: SHINE_COLORS[colorType],
        borderWidth: SHINE_CONFIG.borderWidth,
        duration: SHINE_CONFIG.duration,
    };
}
