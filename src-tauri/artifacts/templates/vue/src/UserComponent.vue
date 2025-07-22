<script setup lang="ts">
import { ref } from 'vue'

const count = ref(0)
const message = ref('Hello Vue!')

const handleClick = () => {
  count.value++
  message.value = `点击了 ${count.value} 次`
}
</script>

<template>
  <div class="user-component">
    <h2>用户组件示例</h2>
    <p>这是一个动态加载的用户组件。</p>
    
    <div class="demo-section">
      <el-button type="primary" @click="handleClick">
        点击我 ({{ count }})
      </el-button>
      <p class="message">{{ message }}</p>
    </div>
    
    <div class="demo-section">
      <el-input v-model="message" placeholder="输入消息" />
    </div>
  </div>
</template>

<style scoped>
.user-component {
  background-color: #f3f4f6;
  border: 1px solid #d1d5db;
  border-radius: 0.5rem;
  padding: 1.5rem;
  text-align: center;
}

.user-component h2 {
  color: #1f2937;
  margin-bottom: 1rem;
}

.user-component p {
  color: #6b7280;
}

.demo-section {
  margin: 1rem 0;
}

.message {
  margin-top: 1rem;
  font-weight: bold;
  color: #059669;
}
</style>