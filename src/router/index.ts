import { createRouter, createWebHashHistory } from "vue-router";

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", redirect: "/mods" },
    { path: "/mods", component: () => import("@/features/mods/ProjectPage.vue"), meta: { title: "MOD 项目" } },
    { path: "/mods/:modId/chat", component: () => import("@/features/chat/ChatPage.vue"), meta: { title: "生成工作台" } },
    { path: "/mods/:modId/versions", component: () => import("@/features/mods/ModManagerPage.vue"), meta: { title: "版本与安装" } },
    { path: "/skills", component: () => import("@/features/skills/SkillsPage.vue"), meta: { title: "Skills" } },
    { path: "/settings", component: () => import("@/features/settings/SettingsPage.vue"), meta: { title: "设置" } },
  ],
});
