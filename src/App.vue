<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import MenuWindow from "./views/MenuWindow.vue";
import RecipeListView from "./views/RecipeListView.vue";
import ConfirmWindow from "./views/ConfirmWindow.vue";
import ToastWindow from "./views/ToastWindow.vue";
import ConfigWindow from "./views/ConfigWindow.vue";

const label = getCurrentWindow().label;
</script>

<template>
  <MenuWindow v-if="label === 'menu'" />
  <RecipeListView v-else-if="label === 'list'" />
  <ConfirmWindow v-else-if="label === 'confirm'" />
  <ToastWindow v-else-if="label === 'toast'" />
  <ConfigWindow v-else />
</template>
