// NEXUS — TWARDY.exe / TW4RDYDEV.
// Host display-stack libraries that must not be shipped inside the AppImage.

export const forbiddenAppImageLibraries = [
  /^libwayland-client\.so(?:\.|$)/,
  /^libwayland-cursor\.so(?:\.|$)/,
  /^libwayland-egl\.so(?:\.|$)/,
  /^libwayland-server\.so(?:\.|$)/,
  /^libxkbcommon\.so(?:\.|$)/,
  /^libxcb-randr\.so(?:\.|$)/,
  /^libxcb-render\.so(?:\.|$)/,
  /^libxcb-shm\.so(?:\.|$)/,
  /^libXau\.so(?:\.|$)/,
  /^libXdmcp\.so(?:\.|$)/,
];

export const isForbiddenAppImageLibrary = (name) =>
  forbiddenAppImageLibraries.some((pattern) => pattern.test(name));
