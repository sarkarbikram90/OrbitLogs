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

import { useEffect, useRef, useState } from 'react';
import * as THREE from 'three';

interface ConjunctionRadarProps {
  selectedSatId: string | null;
}

export function ConjunctionRadar({ selectedSatId }: ConjunctionRadarProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [tcaCountdown, setTcaCountdown] = useState(2700); // 45 mins default
  const [missDistance, setMissDistance] = useState(148.5);
  const [pcValue, setPcValue] = useState(4.82e-4);
  const [isBurnAuthorized, setIsBurnAuthorized] = useState(false);

  useEffect(() => {
    const timer = setInterval(() => {
      setTcaCountdown((prev) => (prev > 0 ? prev - 1 : 0));
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  useEffect(() => {
    if (!containerRef.current) return;
    const width = containerRef.current.clientWidth;
    const height = containerRef.current.clientHeight;

    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0x020409);

    const camera = new THREE.PerspectiveCamera(45, width / height, 0.1, 1000);
    camera.position.set(0, 15, 30);
    camera.lookAt(0, 0, 0);

    const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
    renderer.setSize(width, height);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));

    containerRef.current.appendChild(renderer.domElement);

    // Coordinate Grid
    const grid = new THREE.GridHelper(40, 20, 0x00f0ff, 0x0e172a);
    grid.position.y = -5;
    scene.add(grid);

    // Primary Satellite (Cyber Cyan sphere with glow)
    const primaryGeo = new THREE.SphereGeometry(1.2, 32, 32);
    const primaryMat = new THREE.MeshBasicMaterial({ color: 0x00f0ff, wireframe: true });
    const primaryMesh = new THREE.Mesh(primaryGeo, primaryMat);
    primaryMesh.position.set(-4, 0, 0);
    scene.add(primaryMesh);

    // Primary Covariance Ellipsoid (1σ / 2σ / 3σ transparent bounds)
    const covGeo = new THREE.SphereGeometry(3.5, 24, 24);
    covGeo.scale(1.2, 0.8, 1.5);
    const covMat = new THREE.MeshBasicMaterial({
      color: 0x00f0ff,
      transparent: true,
      opacity: 0.15,
      wireframe: true,
    });
    const covMesh = new THREE.Mesh(covGeo, covMat);
    covMesh.position.set(-4, 0, 0);
    scene.add(covMesh);

    // Secondary Debris Object (Red glowing hazard)
    const secondaryGeo = new THREE.OctahedronGeometry(1.0, 1);
    const secondaryMat = new THREE.MeshBasicMaterial({ color: 0xff2e54, wireframe: true });
    const secondaryMesh = new THREE.Mesh(secondaryGeo, secondaryMat);
    secondaryMesh.position.set(4, 2, -2);
    scene.add(secondaryMesh);

    // Secondary Covariance Ellipsoid
    const secCovGeo = new THREE.SphereGeometry(4.0, 24, 24);
    secCovGeo.scale(1.5, 1.0, 1.8);
    const secCovMat = new THREE.MeshBasicMaterial({
      color: 0xff2e54,
      transparent: true,
      opacity: 0.18,
      wireframe: true,
    });
    const secCovMesh = new THREE.Mesh(secCovGeo, secCovMat);
    secCovMesh.position.set(4, 2, -2);
    scene.add(secCovMesh);

    // Encounter Corridor Line
    const points = [new THREE.Vector3(-12, -2, 5), new THREE.Vector3(12, 4, -9)];
    const lineGeo = new THREE.BufferGeometry().setFromPoints(points);
    const lineMat = new THREE.LineDashedMaterial({ color: 0xffb700, dashSize: 1, gapSize: 0.5 });
    const corridorLine = new THREE.Line(lineGeo, lineMat);
    corridorLine.computeLineDistances();
    scene.add(corridorLine);

    // Animation loop
    let animId: number;
    const animate = () => {
      animId = requestAnimationFrame(animate);
      primaryMesh.rotation.y += 0.01;
      secondaryMesh.rotation.x += 0.015;
      secondaryMesh.rotation.y += 0.01;
      covMesh.rotation.y += 0.005;
      secCovMesh.rotation.y -= 0.005;
      renderer.render(scene, camera);
    };
    animate();

    const handleResize = () => {
      if (!containerRef.current) return;
      const w = containerRef.current.clientWidth;
      const h = containerRef.current.clientHeight;
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      renderer.setSize(w, h);
    };
    window.addEventListener('resize', handleResize);

    const container = containerRef.current;
    return () => {
      window.removeEventListener('resize', handleResize);
      cancelAnimationFrame(animId);
      if (container && renderer.domElement) {
        container.removeChild(renderer.domElement);
      }
      renderer.dispose();
    };
  }, []);

  const formatTca = (sec: number) => {
    const m = Math.floor(sec / 60);
    const s = sec % 60;
    return `T-${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  };

  const handleAuthorize = () => {
    setIsBurnAuthorized(true);
    setMissDistance(6820.0);
    setPcValue(1.2e-8);
  };

  return (
    <div className="glass-panel tech-corner-panel" style={{ height: '100%', padding: '20px', display: 'grid', gridTemplateRows: 'auto 1fr auto', gap: '16px' }}>
      {/* Header */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          <h2 style={{ margin: 0, fontSize: '16px', fontFamily: 'var(--font-display)', color: 'var(--cyber-cyan)', letterSpacing: '1px' }}>
            🛡 3D CONJUNCTION RADAR & COVARIANCE ELLIPSOID VISUALIZER — {selectedSatId || 'SAT-0001'}
          </h2>
          <span style={{ fontSize: '11px', color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>
            Interactive Three.js Conjunction Corridor, 3D 3σ Uncertainty Ellipsoids & RIC Vector Calculation
          </span>
        </div>
        <div style={{ backgroundColor: 'rgba(255, 46, 84, 0.15)', border: '1px solid var(--status-red)', padding: '6px 14px', borderRadius: '6px', textAlign: 'right' }}>
          <span style={{ fontSize: '9px', color: 'var(--text-muted)', display: 'block', fontFamily: 'var(--font-display)' }}>TCA COUNTDOWN</span>
          <span style={{ fontSize: '18px', fontWeight: 900, color: 'var(--status-red)', fontFamily: 'var(--font-mono)' }}>
            {formatTca(tcaCountdown)}
          </span>
        </div>
      </div>

      {/* 3D Canvas Canvas Viewport */}
      <div style={{ position: 'relative', width: '100%', height: '100%', borderRadius: '8px', overflow: 'hidden', border: '1px solid var(--border-color)' }}>
        <div ref={containerRef} style={{ width: '100%', height: '100%' }} />

        {/* Reticle HUD overlay */}
        <div style={{ position: 'absolute', top: '16px', left: '16px', backgroundColor: 'rgba(3, 7, 16, 0.85)', border: '1px solid var(--border-color)', borderRadius: '6px', padding: '12px', fontFamily: 'var(--font-mono)', fontSize: '11px', lineHeight: '1.6' }}>
          <div style={{ color: 'var(--cyber-cyan)', fontWeight: 'bold', marginBottom: '4px' }}>OBJECT A: {selectedSatId || 'SAT-0001'} (PRIMARY)</div>
          <div style={{ color: 'var(--text-secondary)' }}>Pos: [6800.0, 0.0, 0.0] km | Cov: 100m (1σ)</div>
          <div style={{ color: 'var(--status-red)', fontWeight: 'bold', marginTop: '8px', marginBottom: '4px' }}>OBJECT B: COSMOS-2251-DEB-1842 (SECONDARY)</div>
          <div style={{ color: 'var(--text-secondary)' }}>Type: DEBRIS | Cov: 200m (1σ) | Rel Vel: 14.2 km/s</div>
        </div>

        {/* Status metrics HUD overlay */}
        <div style={{ position: 'absolute', bottom: '16px', right: '16px', backgroundColor: 'rgba(3, 7, 16, 0.85)', border: '1px solid var(--border-color)', borderRadius: '6px', padding: '12px', fontFamily: 'var(--font-mono)', fontSize: '11px' }}>
          <div style={{ display: 'flex', gap: '20px' }}>
            <div>
              <span style={{ color: 'var(--text-muted)' }}>Miss Distance:</span>
              <div style={{ fontSize: '16px', fontWeight: 'bold', color: missDistance < 500 ? 'var(--status-red)' : 'var(--status-green)' }}>
                {missDistance.toFixed(1)} m
              </div>
            </div>
            <div>
              <span style={{ color: 'var(--text-muted)' }}>Collision Prob (Pc):</span>
              <div style={{ fontSize: '16px', fontWeight: 'bold', color: pcValue > 1e-4 ? 'var(--status-red)' : 'var(--status-green)' }}>
                {pcValue.toExponential(2)}
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* 1-Click Autonomous Maneuver Bar */}
      <div style={{ backgroundColor: 'rgba(6, 12, 24, 0.8)', border: '1px solid var(--border-color)', borderRadius: '8px', padding: '16px', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div style={{ fontFamily: 'var(--font-mono)', fontSize: '12px' }}>
          <span style={{ color: 'var(--cyber-cyan)', fontWeight: 'bold' }}>AI CAM PROPOSAL: </span>
          <span>Radial: +0.00 m/s | In-Track: +0.45 m/s | Cross-Track: +0.02 m/s (Propellant: 0.42 kg)</span>
        </div>
        <button
          onClick={handleAuthorize}
          disabled={isBurnAuthorized}
          className="cyber-btn"
          style={{
            backgroundColor: isBurnAuthorized ? 'rgba(0, 255, 157, 0.2)' : 'var(--status-red)',
            borderColor: isBurnAuthorized ? 'var(--status-green)' : 'var(--status-red)',
            color: isBurnAuthorized ? 'var(--status-green)' : '#fff',
          }}
        >
          {isBurnAuthorized ? '✓ MANEUVER AUTHORIZED & UPLINKED' : '⚡ 1-CLICK AUTHORIZE CAM BURN'}
        </button>
      </div>
    </div>
  );
}
