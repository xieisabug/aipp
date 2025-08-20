"use client";

import * as React from "react";

import { cn } from "@/utils/utils";

interface ShineBorderProps extends React.HTMLAttributes<HTMLDivElement> {
    /**
     * Width of the border in pixels
     * @default 1
     */
    borderWidth?: number;
    /**
     * Duration of the animation in seconds
     * @default 14
     */
    duration?: number;
    /**
     * Color of the border, can be:
     * - A single color string (hex, hsl, rgb, CSS variable)
     * - An array of colors for gradient effect
     * - Tailwind CSS variable format (e.g., 'var(--shine-primary)')
     * @default "#000000"
     */
    shineColor?: string | string[];
}

/**
 * 处理颜色值，支持 CSS 变量和直接颜色值
 */
function processColor(color: string): string {
    // 如果是 CSS 变量格式，需要用 hsl() 包装
    if (color.startsWith("var(--")) {
        return `hsl(${color})`;
    }
    // 其他格式直接返回
    return color;
}

/**
 * Shine Border
 *
 * An animated background border effect component with configurable properties.
 * Supports both direct color values and Tailwind CSS variables.
 */
export function ShineBorder({
    borderWidth = 1,
    duration = 14,
    shineColor = "#000000",
    className,
    style,
    ...props
}: ShineBorderProps) {
    // 处理颜色配置
    const processedColors = Array.isArray(shineColor)
        ? shineColor.map(processColor).join(",")
        : processColor(shineColor);

    return (
        <div
            style={
                {
                    "--border-width": `${borderWidth}px`,
                    "--duration": `${duration}s`,
                    backgroundImage: `radial-gradient(transparent,transparent, ${processedColors},transparent,transparent)`,
                    backgroundSize: "300% 300%",
                    mask: `linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0)`,
                    WebkitMask: `linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0)`,
                    WebkitMaskComposite: "xor",
                    maskComposite: "exclude",
                    padding: "var(--border-width)",
                    ...style,
                } as React.CSSProperties
            }
            className={cn(
                "pointer-events-none absolute inset-0 size-full rounded-[inherit] will-change-[background-position] motion-safe:animate-shine",
                className
            )}
            {...props}
        />
    );
}
