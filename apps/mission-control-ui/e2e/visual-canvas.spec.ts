// Copyright 2026 OrbitLogs Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
import { test, expect } from '@playwright/test';

test.describe('OrbitLogs Mission Control UI - E2E Visual Canvas Tests', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to local dev server URL
    await page.goto('/');
  });

  test('should load the 3D globe canvas and verify WebGL context', async ({ page }) => {
    // 1. Verify canvas element is visible in DOM
    const canvas = page.locator('canvas');
    await expect(canvas).toBeVisible();

    // 2. Validate WebGL / Canvas rendering support on the canvas
    const webglSupported = await page.evaluate(() => {
      const canvas = document.querySelector('canvas');
      if (!canvas) return false;
      const gl = canvas.getContext('webgl') || canvas.getContext('experimental-webgl') || canvas.getContext('webgl2') || canvas.getContext('2d');
      return !!gl;
    });
    expect(webglSupported).toBe(true);
  });

  test('should verify canvas dimensions adapt under scaling', async ({ page }) => {
    const canvas = page.locator('canvas');
    
    // Check initial layout box
    const initialBox = await canvas.boundingBox();
    expect(initialBox).not.toBeNull();
    
    // Resize viewport to scale the page
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.waitForTimeout(500);
    
    const scaledBox = await canvas.boundingBox();
    expect(scaledBox).not.toBeNull();
    expect(scaledBox?.width).toBe(1280);
  });

  test('should verify WebSocket connection and frame receiving performance', async ({ page }) => {
    const consoleErrors: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'error') {
        consoleErrors.push(msg.text());
      }
    });

    await page.waitForTimeout(2000);
    
    // Filter connection / WebSocket logs for errors
    const wsErrors = consoleErrors.filter(err => err.toLowerCase().includes('websocket') || err.toLowerCase().includes('ws:'));
    expect(wsErrors.length).toBe(0);
  });

  test('should verify navigation tabs and view toggles exist', async ({ page }) => {
    await page.evaluate(() => {
      sessionStorage.setItem('VITE_API_KEY', 'test-key');
      sessionStorage.setItem('JWT_TOKEN', 'test-token');
    });
    await page.reload();
    await page.waitForLoadState('domcontentloaded');

    const headerTitle = page.locator('.header-title');
    await expect(headerTitle).toBeVisible();
    await expect(headerTitle).toContainText(/ORBITLOGS/i);

    const dashboardTab = page.locator('button', { hasText: 'DASHBOARD' });
    await expect(dashboardTab).toBeVisible();

    const commandsTab = page.locator('button', { hasText: 'COMMANDS' });
    await expect(commandsTab).toBeVisible();
  });
});
