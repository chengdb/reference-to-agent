<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import MenuApp from "./MenuApp.vue";
import ConfigApp from "./ConfigApp.vue";

const label = getCurrentWindow().label;
</script>

<template>
  <MenuApp v-if="label === 'menu'" />
  <ConfigApp v-else />
</template>
