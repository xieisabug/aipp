<script setup lang="ts">
import { ref, onMounted, defineAsyncComponent } from 'vue'

const error = ref<string | null>(null)
const loading = ref(true)

// 使用 defineAsyncComponent 创建异步组件
const DynamicComponent = defineAsyncComponent({
  loader: () => import('./UserComponent.vue'),
  loadingComponent: {
    template: '<div class="loading-spinner"></div>'
  },
  errorComponent: {
    template: '<div class="error-message">组件加载失败</div>'
  },
  delay: 200,
  timeout: 10000,
  onError: (error) => {
    console.error('Failed to load user component:', error)
  }
})

onMounted(() => {
  // 组件加载完成后隐藏加载状态
  setTimeout(() => {
    loading.value = false
  }, 100)
})
</script>

<template>
  <div class="container">
    <div class="component-preview">
      <!-- 直接渲染动态组件，defineAsyncComponent 会自动处理加载和错误状态 -->
      <DynamicComponent />
    </div>
  </div>
</template>

<style scoped>
.container {
  width: 100%;
  margin: 0 auto;
  padding: 2rem;
}

.component-preview {
  width: 100%;
}

/* 为 defineAsyncComponent 的加载和错误状态提供样式 */
:deep(.loading-spinner) {
  width: 2rem;
  height: 2rem;
  border: 2px solid #f3f4f6;
  border-top: 2px solid #2563eb;
  border-radius: 50%;
  animation: spin 1s linear infinite;
  margin: 2rem auto;
}

:deep(.error-message) {
  color: #ef4444;
  text-align: center;
  padding: 2rem;
  background-color: #fef2f2;
  border: 1px solid #fecaca;
  border-radius: 0.5rem;
  margin: 2rem;
}

@keyframes spin {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}
</style>
