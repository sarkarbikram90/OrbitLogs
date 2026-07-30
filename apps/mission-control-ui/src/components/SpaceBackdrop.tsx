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

import React, { useEffect, useRef } from 'react';

interface Star {
  x: number;
  y: number;
  z: number;
  size: number;
  alpha: number;
  alphaSpeed: number;
}

interface SatelliteNode {
  x: number;
  y: number;
  vx: number;
  vy: number;
  label: string;
  color: string;
}

export const SpaceBackdrop: React.FC = () => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    let animationFrameId: number;
    let width = (canvas.width = window.innerWidth);
    let height = (canvas.height = window.innerHeight);

    const handleResize = () => {
      if (!canvas) return;
      width = canvas.width = window.innerWidth;
      height = canvas.height = window.innerHeight;
    };
    window.addEventListener('resize', handleResize);

    // Mouse tracking for subtle interactive star drift
    let mouseX = width / 2;
    let mouseY = height / 2;
    const handleMouseMove = (e: MouseEvent) => {
      mouseX = e.clientX;
      mouseY = e.clientY;
    };
    window.addEventListener('mousemove', handleMouseMove);

    // Generate 250 Stars with 3D Depth
    const stars: Star[] = Array.from({ length: 250 }, () => ({
      x: Math.random() * width,
      y: Math.random() * height,
      z: Math.random() * 2 + 0.5,
      size: Math.random() * 1.8 + 0.3,
      alpha: Math.random(),
      alphaSpeed: (Math.random() * 0.02 + 0.005) * (Math.random() > 0.5 ? 1 : -1),
    }));

    // Generate floating satellite constellation nodes
    const satNodes: SatelliteNode[] = [
      { x: width * 0.15, y: height * 0.25, vx: 0.2, vy: 0.1, label: 'SAT-0001 (LEO)', color: '#00f0ff' },
      { x: width * 0.75, y: height * 0.35, vx: -0.15, vy: 0.12, label: 'SAT-0002 (MEO)', color: '#00ff9d' },
      { x: width * 0.4, y: height * 0.7, vx: 0.1, vy: -0.18, label: 'SAT-0003 (GEO)', color: '#ffb700' },
      { x: width * 0.85, y: height * 0.8, vx: -0.22, vy: -0.1, label: 'SAT-0004 (LEO)', color: '#00f0ff' },
    ];

    let gridRotation = 0;

    const render = () => {
      // Clear with ultra-deep gradient
      const bgGradient = ctx.createRadialGradient(
        width / 2,
        height / 2,
        0,
        width / 2,
        height / 2,
        Math.max(width, height) * 0.8
      );
      bgGradient.addColorStop(0, '#040914');
      bgGradient.addColorStop(0.5, '#02050c');
      bgGradient.addColorStop(1, '#010205');
      ctx.fillStyle = bgGradient;
      ctx.fillRect(0, 0, width, height);

      // Draw faint tactical background grid lines
      ctx.strokeStyle = 'rgba(0, 240, 255, 0.025)';
      ctx.lineWidth = 1;
      const gridSize = 80;
      for (let x = 0; x < width; x += gridSize) {
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, height);
        ctx.stroke();
      }
      for (let y = 0; y < height; y += gridSize) {
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(width, y);
        ctx.stroke();
      }

      // Render Stars with subtle cursor shift
      const shiftX = (mouseX - width / 2) * 0.015;
      const shiftY = (mouseY - height / 2) * 0.015;

      stars.forEach((star) => {
        star.alpha += star.alphaSpeed;
        if (star.alpha <= 0.1 || star.alpha >= 1) {
          star.alphaSpeed = -star.alphaSpeed;
        }

        const renderX = star.x + shiftX * star.z;
        const renderY = star.y + shiftY * star.z;

        ctx.fillStyle = `rgba(255, 255, 255, ${star.alpha.toFixed(2)})`;
        ctx.beginPath();
        ctx.arc(renderX, renderY, star.size, 0, Math.PI * 2);
        ctx.fill();

        // Subtle glow for larger stars
        if (star.size > 1.2) {
          ctx.fillStyle = `rgba(0, 240, 255, ${(star.alpha * 0.3).toFixed(2)})`;
          ctx.beginPath();
          ctx.arc(renderX, renderY, star.size * 2.5, 0, Math.PI * 2);
          ctx.fill();
        }
      });

      // Render Constellation Link Lines between nodes
      ctx.strokeStyle = 'rgba(0, 240, 255, 0.15)';
      ctx.lineWidth = 1;
      ctx.setLineDash([4, 4]);
      ctx.beginPath();
      for (let i = 0; i < satNodes.length; i++) {
        const next = satNodes[(i + 1) % satNodes.length];
        ctx.moveTo(satNodes[i].x, satNodes[i].y);
        ctx.lineTo(next.x, next.y);
      }
      ctx.stroke();
      ctx.setLineDash([]);

      // Update & Render Satellite Nodes
      satNodes.forEach((node) => {
        node.x += node.vx;
        node.y += node.vy;

        if (node.x < 50 || node.x > width - 50) node.vx = -node.vx;
        if (node.y < 50 || node.y > height - 50) node.vy = -node.vy;

        // Node Glow Ring
        ctx.fillStyle = node.color;
        ctx.shadowColor = node.color;
        ctx.shadowBlur = 10;
        ctx.beginPath();
        ctx.arc(node.x, node.y, 4, 0, Math.PI * 2);
        ctx.fill();
        ctx.shadowBlur = 0;

        // Outer pulsing ring
        ctx.strokeStyle = node.color;
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.arc(node.x, node.y, 10 + Math.sin(Date.now() * 0.003) * 3, 0, Math.PI * 2);
        ctx.stroke();

        // Label
        ctx.fillStyle = 'rgba(148, 163, 184, 0.7)';
        ctx.font = '9px "JetBrains Mono", monospace';
        ctx.fillText(node.label, node.x + 14, node.y + 3);
      });

      // Draw Horizon Orbital Grid Curve at bottom
      gridRotation += 0.002;
      ctx.strokeStyle = 'rgba(0, 240, 255, 0.08)';
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.ellipse(width / 2, height + 400, width * 0.9, 500, 0, 0, Math.PI * 2);
      ctx.stroke();

      animationFrameId = requestAnimationFrame(render);
    };

    render();

    return () => {
      cancelAnimationFrame(animationFrameId);
      window.removeEventListener('resize', handleResize);
      window.removeEventListener('mousemove', handleMouseMove);
    };
  }, []);

  return (
    <canvas
      ref={canvasRef}
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        width: '100vw',
        height: '100vh',
        pointerEvents: 'none',
        zIndex: 0,
      }}
    />
  );
};
