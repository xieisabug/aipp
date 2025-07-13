import "./index.css";
import React, { Suspense, useEffect, useState } from "react";
import { Card, CardContent } from "@/components/ui/card";

export function App() {
  const [DynamicComponent, setDynamicComponent] = useState<React.ComponentType | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // 动态加载用户组件
    const loadUserComponent = async () => {
      try {
        // 尝试加载 UserComponent.tsx
        const module = await import('./UserComponent');
        setDynamicComponent(() => module.default || module.UserComponent);
      } catch (err) {
        console.error('Failed to load user component:', err);
        setError('Failed to load component');
      }
    };

    loadUserComponent();
  }, []);

  if (error) {
    return (
      <div className="container mx-auto p-8 text-center">
        <Card className="bg-red-50 border-red-200">
          <CardContent className="pt-6">
            <h1 className="text-3xl font-bold text-red-600 mb-4">组件加载失败</h1>
            <p className="text-red-500">{error}</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="container mx-auto p-8">
      <div className="component-preview">
        {DynamicComponent ? (
          <Suspense fallback={
            <Card>
              <CardContent className="pt-6">
                <div className="text-center">
                  <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600 mx-auto mb-2"></div>
                  <p>加载组件中...</p>
                </div>
              </CardContent>
            </Card>
          }>
            <DynamicComponent />
          </Suspense>
        ) : (
          <Card>
            <CardContent className="pt-6">
              <div className="text-center text-gray-500">
                <p>未找到组件。请确保组件存在。</p>
              </div>
            </CardContent>
          </Card>
        )}
      </div>
    </div>
  );
}

export default App;
