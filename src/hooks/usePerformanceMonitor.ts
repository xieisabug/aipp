import { useEffect, useRef } from 'react';

// 性能监控 Hook
export const usePerformanceMonitor = (componentName: string, deps?: any[]) => {
    const renderStartRef = useRef<number>(0);
    const mountStartRef = useRef<number>(0);
    const isFirstRenderRef = useRef(true);

    // 在每次渲染开始时记录时间
    renderStartRef.current = performance.now();

    useEffect(() => {
        const renderTime = performance.now() - renderStartRef.current;
        
        if (isFirstRenderRef.current) {
            // 首次渲染（挂载）
            isFirstRenderRef.current = false;
            const mountTime = performance.now() - mountStartRef.current;
            
            console.log(`🚀 [${componentName}] Mount:`, {
                mountTime: `${mountTime.toFixed(2)}ms`,
                renderTime: `${renderTime.toFixed(2)}ms`,
            });
            
            // 发送性能数据到控制台或监控系统
            if (typeof window !== 'undefined' && window.performance && typeof window.performance.mark === 'function') {
                performance.mark(`${componentName}-mount-start`);
                performance.mark(`${componentName}-mount-end`);
                performance.measure(`${componentName}-mount`, `${componentName}-mount-start`, `${componentName}-mount-end`);
            }
        } else {
            // 更新渲染
            console.log(`🔄 [${componentName}] Update:`, {
                renderTime: `${renderTime.toFixed(2)}ms`,
                reason: deps ? 'deps changed' : 'props changed'
            });
            
            // 记录更新性能
            if (typeof window !== 'undefined' && window.performance && typeof window.performance.mark === 'function') {
                performance.mark(`${componentName}-update-${Date.now()}`);
            }
        }
        
        // 如果渲染时间过长，发出警告
        if (renderTime > 16.67) { // 60fps = 16.67ms per frame
            console.warn(`⚠️  [${componentName}] Slow render: ${renderTime.toFixed(2)}ms (>16.67ms)`);
        }
    }, deps);

    // 在组件挂载时记录开始时间
    if (isFirstRenderRef.current) {
        mountStartRef.current = performance.now();
    }

    return {
        logCustomMetric: (metricName: string, value: number) => {
            console.log(`📊 [${componentName}] ${metricName}:`, `${value.toFixed(2)}ms`);
        }
    };
};

// React DevTools Profiler 的回调函数
export const onRenderCallback = (
    id: string,
    phase: 'mount' | 'update',
    actualDuration: number,
    baseDuration: number,
    startTime: number,
    commitTime: number,
    interactions: Set<any>
) => {
    console.log(`📈 [Profiler-${id}] ${phase}:`, {
        actualDuration: `${actualDuration.toFixed(2)}ms`,
        baseDuration: `${baseDuration.toFixed(2)}ms`,
        startTime: `${startTime.toFixed(2)}ms`,
        commitTime: `${commitTime.toFixed(2)}ms`,
        interactions: interactions.size,
    });

    // 性能阈值警告
    if (actualDuration > 50) {
        console.warn(`⚠️  [Profiler-${id}] Performance issue: ${actualDuration.toFixed(2)}ms`);
    }
};

// 测量特定操作的性能
export const measureAsync = async <T>(
    operationName: string,
    operation: () => Promise<T>
): Promise<T> => {
    const start = performance.now();
    try {
        const result = await operation();
        const duration = performance.now() - start;
        console.log(`⏱️  [${operationName}] completed:`, `${duration.toFixed(2)}ms`);
        return result;
    } catch (error) {
        const duration = performance.now() - start;
        console.error(`❌ [${operationName}] failed after:`, `${duration.toFixed(2)}ms`, error);
        throw error;
    }
};

// 同步操作性能测量
export const measureSync = <T>(
    operationName: string,
    operation: () => T
): T => {
    const start = performance.now();
    try {
        const result = operation();
        const duration = performance.now() - start;
        console.log(`⏱️  [${operationName}] completed:`, `${duration.toFixed(2)}ms`);
        return result;
    } catch (error) {
        const duration = performance.now() - start;
        console.error(`❌ [${operationName}] failed after:`, `${duration.toFixed(2)}ms`, error);
        throw error;
    }
};

// 内存使用监控
export const logMemoryUsage = (componentName: string) => {
    if (window.performance && (window.performance as any).memory) {
        const memory = (window.performance as any).memory;
        console.log(`🧠 [${componentName}] Memory:`, {
            used: `${(memory.usedJSHeapSize / 1024 / 1024).toFixed(2)}MB`,
            total: `${(memory.totalJSHeapSize / 1024 / 1024).toFixed(2)}MB`,
            limit: `${(memory.jsHeapSizeLimit / 1024 / 1024).toFixed(2)}MB`,
        });
    }
};