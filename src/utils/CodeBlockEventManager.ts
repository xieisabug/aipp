// 全局 CodeBlock 事件管理器，减少重复的事件监听器
class CodeBlockEventManager {
    private static instance: CodeBlockEventManager | null = null;
    private codeBlocks = new Map<HTMLDivElement, {
        onScroll: () => void;
        onMouseMove: (e: MouseEvent) => void;
    }>();
    private isListening = false;
    private mousePosition = { x: 0, y: 0 };

    static getInstance(): CodeBlockEventManager {
        if (!CodeBlockEventManager.instance) {
            CodeBlockEventManager.instance = new CodeBlockEventManager();
        }
        return CodeBlockEventManager.instance;
    }

    private throttle<T extends any[]>(func: (...args: T) => void, delay: number) {
        let timeoutId: NodeJS.Timeout | null = null;
        return (...args: T) => {
            if (timeoutId) return;
            timeoutId = setTimeout(() => {
                func(...args);
                timeoutId = null;
            }, delay);
        };
    }

    private handleGlobalMouseMove = (e: MouseEvent) => {
        this.mousePosition = { x: e.clientX, y: e.clientY };
        // 通知所有注册的 CodeBlock
        this.codeBlocks.forEach((handlers) => {
            handlers.onMouseMove(e);
        });
    };

    private handleGlobalScroll = this.throttle(() => {
        // 通知所有注册的 CodeBlock
        this.codeBlocks.forEach((handlers) => {
            handlers.onScroll();
        });
    }, 100);

    private startListening() {
        if (this.isListening) return;
        
        const events = ['scroll', 'wheel', 'touchmove'];
        events.forEach(event => {
            window.addEventListener(event, this.handleGlobalScroll, { passive: true });
        });
        
        window.addEventListener('mousemove', this.handleGlobalMouseMove, { passive: true });
        this.isListening = true;
    }

    private stopListening() {
        if (!this.isListening) return;
        
        const events = ['scroll', 'wheel', 'touchmove'];
        events.forEach(event => {
            window.removeEventListener(event, this.handleGlobalScroll);
        });
        
        window.removeEventListener('mousemove', this.handleGlobalMouseMove);
        this.isListening = false;
    }

    register(
        element: HTMLDivElement,
        handlers: {
            onScroll: () => void;
            onMouseMove: (e: MouseEvent) => void;
        }
    ) {
        this.codeBlocks.set(element, handlers);
        if (this.codeBlocks.size === 1) {
            this.startListening();
        }
        
        // 初始调用
        handlers.onScroll();
    }

    unregister(element: HTMLDivElement) {
        this.codeBlocks.delete(element);
        if (this.codeBlocks.size === 0) {
            this.stopListening();
        }
    }

    getMousePosition() {
        return this.mousePosition;
    }
}

export default CodeBlockEventManager;