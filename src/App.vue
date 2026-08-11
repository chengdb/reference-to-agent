<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import MenuWindow from "./views/MenuWindow.vue";
import ConfigWindow from "./views/ConfigWindow.vue";

const label = getCurrentWindow().label;
</script>

<template>
  <MenuWindow v-if="label === 'menu'" />
  <ConfigWindow v-else />
</template>
