import React, { useEffect, useRef } from 'react';
import * as THREE from 'three';
import type { Satellite, GroundStation } from '../types';

interface OrbitViewerProps {
  satellites: Satellite[];
  groundStations: GroundStation[];
  selectedSatId: string | null;
}

// Convert Lat/Lon to 3D coordinates on sphere
function latLonToVector(lat: number, lon: number, radius: number): THREE.Vector3 {
  const phi = (90 - lat) * (Math.PI / 180);
  const theta = (lon + 180) * (Math.PI / 180);
  
  const x = -(radius * Math.sin(phi) * Math.sin(theta));
  const y = radius * Math.cos(phi);
  const z = radius * Math.sin(phi) * Math.cos(theta);
  
  return new THREE.Vector3(x, y, z);
}

function propagateKepler(
  a: number,
  e: number,
  inclination: number,
  raan: number,
  argOfPerigee: number,
  meanMotion: number,
  meanAnomalyEpoch: number,
  t: number
): THREE.Vector3 {
  // Mean anomaly
  const M = meanAnomalyEpoch + meanMotion * t;
  
  // Solve Kepler's equation: E - e*sin(E) = M
  let E = M;
  for (let step = 0; step < 5; step++) {
    E = E - (E - e * Math.sin(E) - M) / (1 - e * Math.cos(E));
  }
  
  // Perifocal coordinates
  const x_prime = a * (Math.cos(E) - e);
  const y_prime = a * Math.sqrt(1 - e * e) * Math.sin(E);
  
  // Rotation coefficients
  const cosO = Math.cos(raan);
  const sinO = Math.sin(raan);
  const cosw = Math.cos(argOfPerigee);
  const sinw = Math.sin(argOfPerigee);
  const cosi = Math.cos(inclination);
  const sini = Math.sin(inclination);
  
  const px = cosw * cosO - sinw * sinO * cosi;
  const py = cosw * sinO + sinw * cosO * cosi;
  const pz = sinw * sini;
  
  const qx = -sinw * cosO - cosw * sinO * cosi;
  const qy = -sinw * sinO + cosw * cosO * cosi;
  const qz = cosw * sini;
  
  const x_eci = x_prime * px + y_prime * qx;
  const y_eci = x_prime * py + y_prime * qy;
  const z_eci = x_prime * pz + y_prime * qz;
  
  // Map ECI to ThreeJS (y is up, z is out of screen)
  return new THREE.Vector3(x_eci, z_eci, y_eci);
}

export const OrbitViewer: React.FC<OrbitViewerProps> = ({ satellites, groundStations, selectedSatId }) => {
  const mountRef = useRef<HTMLDivElement>(null);
  
  // Track props in refs so the animation loop can access their latest values without re-triggering useEffect
  const satellitesRef = useRef<Satellite[]>(satellites);
  const groundStationsRef = useRef<GroundStation[]>(groundStations);
  const selectedSatIdRef = useRef<string | null>(selectedSatId);

  useEffect(() => {
    satellitesRef.current = satellites;
  }, [satellites]);

  useEffect(() => {
    groundStationsRef.current = groundStations;
  }, [groundStations]);

  useEffect(() => {
    selectedSatIdRef.current = selectedSatId;
  }, [selectedSatId]);

  useEffect(() => {
    if (!mountRef.current) return;

    const container = mountRef.current;
    const width = container.clientWidth;
    const height = container.clientHeight;

    // 1. Scene setup
    const scene = new THREE.Scene();
    
    // 2. Camera setup
    const camera = new THREE.PerspectiveCamera(50, width / height, 0.1, 1000);
    camera.position.z = 8;
    camera.position.y = 3;

    // 3. Renderer setup
    const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
    renderer.setSize(width, height);
    renderer.setPixelRatio(window.devicePixelRatio);
    container.appendChild(renderer.domElement);

    // 4. Earth creation (Sleek dark wireframe with a translucent core)
    const earthRadius = 2.0;
    
    // Earth Core
    const coreGeo = new THREE.SphereGeometry(earthRadius - 0.02, 32, 32);
    const coreMat = new THREE.MeshBasicMaterial({
      color: 0x050c18,
      transparent: true,
      opacity: 0.8
    });
    const earthCore = new THREE.Mesh(coreGeo, coreMat);
    scene.add(earthCore);

    // Earth Grid Wireframe
    const gridGeo = new THREE.SphereGeometry(earthRadius, 32, 16);
    const gridMat = new THREE.MeshBasicMaterial({
      color: 0x1e3a8a,
      wireframe: true,
      transparent: true,
      opacity: 0.4
    });
    const earthGrid = new THREE.Mesh(gridGeo, gridMat);
    scene.add(earthGrid);

    // Add equator ring for visual reference
    const equatorGeo = new THREE.RingGeometry(earthRadius + 0.01, earthRadius + 0.03, 64);
    const equatorMat = new THREE.MeshBasicMaterial({
      color: 0x3b82f6,
      side: THREE.DoubleSide,
      transparent: true,
      opacity: 0.2
    });
    const equator = new THREE.Mesh(equatorGeo, equatorMat);
    equator.rotation.x = Math.PI / 2;
    scene.add(equator);

    // 5. Ground Stations points mapping
    const gsLocations = [
      { id: 'GS-0001', name: 'Svalbard', lat: 78.2, lon: 15.6 },
      { id: 'GS-0002', name: 'Troll', lat: -72.0, lon: 2.5 },
      { id: 'GS-0003', name: 'Kiruna', lat: 67.8, lon: 20.2 },
      { id: 'GS-0004', name: 'McMurdo', lat: -77.8, lon: 166.7 }
    ];

    const gsMeshes: { id: string; mesh: THREE.Mesh }[] = [];
    gsLocations.forEach(gs => {
      const pos = latLonToVector(gs.lat, gs.lon, earthRadius);
      const gsGeo = new THREE.BoxGeometry(0.08, 0.08, 0.08);
      
      const gsMat = new THREE.MeshBasicMaterial({ color: 0x10b981 });
      const mesh = new THREE.Mesh(gsGeo, gsMat);
      mesh.position.copy(pos);
      // Orient Box to face outwards
      mesh.lookAt(new THREE.Vector3(0,0,0));
      scene.add(mesh);
      gsMeshes.push({ id: gs.id, mesh });
    });

    // 6. Satellites orbits mapping
    const satData = Array.from({ length: 10 }).map((_, idx) => {
      const id = `SAT-${String(idx + 1).padStart(4, '0')}`;
      const a = 2.8 + (idx * 0.15); // Semi-major axis
      const e = 0.03 + (idx * 0.012); // Eccentricity
      const inclination = (45 + idx * 5) * (Math.PI / 180); // Inclination
      const raan = (idx * 36) * (Math.PI / 180); // RAAN
      const argOfPerigee = (idx * 30) * (Math.PI / 180); // Argument of Perigee
      const meanMotion = 0.4 + (10 - idx) * 0.04; // Mean Motion
      const meanAnomalyEpoch = (idx * 45) * (Math.PI / 180); // Mean Anomaly at epoch
      const linkedGsId = `GS-000${(idx % 4) + 1}`;
      return { id, a, e, inclination, raan, argOfPerigee, meanMotion, meanAnomalyEpoch, linkedGsId };
    });

    const satMeshes: { id: string; mesh: THREE.Mesh; line: THREE.Line; orbit: THREE.Line }[] = [];

    satData.forEach(sat => {
      // Create Orbit Ring Loop
      const orbitPoints: THREE.Vector3[] = [];
      const period = (2 * Math.PI) / sat.meanMotion;
      for (let i = 0; i <= 64; i++) {
        const t = (i / 64) * period;
        const pt = propagateKepler(
          sat.a,
          sat.e,
          sat.inclination,
          sat.raan,
          sat.argOfPerigee,
          sat.meanMotion,
          sat.meanAnomalyEpoch,
          t
        );
        orbitPoints.push(pt);
      }
      const orbitGeo = new THREE.BufferGeometry().setFromPoints(orbitPoints);
      
      const orbitMat = new THREE.LineBasicMaterial({
        color: 0x1e293b,
        transparent: true,
        opacity: 0.3
      });
      const orbit = new THREE.Line(orbitGeo, orbitMat);
      scene.add(orbit);

      // Create Satellite mesh
      const satGeo = new THREE.SphereGeometry(0.05, 8, 8);
      const satMat = new THREE.MeshBasicMaterial({ color: 0x10b981 });
      const mesh = new THREE.Mesh(satGeo, satMat);
      scene.add(mesh);

      // Create Connection Beam line (initially empty)
      const beamGeo = new THREE.BufferGeometry().setFromPoints([
        new THREE.Vector3(0,0,0),
        new THREE.Vector3(0,0,0)
      ]);
      const beamMat = new THREE.LineDashedMaterial({
        color: 0x0ea5e9,
        dashSize: 0.1,
        gapSize: 0.1,
        transparent: true,
        opacity: 0.2
      });
      const line = new THREE.Line(beamGeo, beamMat);
      scene.add(line);

      satMeshes.push({ id: sat.id, mesh, line, orbit });
    });

    // 7. Animation tick loop
    const clock = new THREE.Clock();
    let animationId: number;

    const animate = () => {
      animationId = requestAnimationFrame(animate);
      const elapsed = clock.getElapsedTime();

      // Rotate Earth grid slowly
      earthGrid.rotation.y = elapsed * 0.05;
      earthCore.rotation.y = elapsed * 0.05;

      const currentSatellites = satellitesRef.current;
      const currentGroundStations = groundStationsRef.current;
      const currentSelectedSatId = selectedSatIdRef.current;

      // Update Ground Station status colors from ref
      gsLocations.forEach((gs, idx) => {
        const liveGs = currentGroundStations.find(g => g.station_id === gs.id);
        const status = liveGs ? liveGs.status : 'ONLINE';
        
        let color = 0x10b981; // GREEN
        if (status === 'DEGRADED') color = 0xf59e0b; // YELLOW
        else if (status === 'OUTAGE') color = 0xef4444; // RED

        (gsMeshes[idx].mesh.material as THREE.MeshBasicMaterial).color.setHex(color);
      });

      // Update Satellites positions & Signal Beam connections
      satData.forEach((sat, idx) => {
        const satMesh = satMeshes[idx];
        
        // Calculate Satellite position using Keplerian propagator
        const satPos = propagateKepler(
          sat.a,
          sat.e,
          sat.inclination,
          sat.raan,
          sat.argOfPerigee,
          sat.meanMotion,
          sat.meanAnomalyEpoch,
          elapsed
        );
        satMesh.mesh.position.copy(satPos);

        // Update colors based on live inputs from ref
        const liveSat = currentSatellites.find(s => s.id === sat.id);
        const state = liveSat ? liveSat.state : 'GREEN';
        let color = 0x10b981; // GREEN
        if (state === 'YELLOW') color = 0xf59e0b; // YELLOW
        else if (state === 'RED') color = 0xef4444; // RED
        (satMesh.mesh.material as THREE.MeshBasicMaterial).color.setHex(color);

        const isSelected = currentSelectedSatId === sat.id;
        
        // Scale selected satellite slightly
        const targetScale = isSelected ? 1.6 : 1.0;
        satMesh.mesh.scale.set(targetScale, targetScale, targetScale);

        (satMesh.orbit.material as THREE.LineBasicMaterial).color.setHex(isSelected ? 0x0ea5e9 : 0x1e293b);
        (satMesh.orbit.material as THREE.LineBasicMaterial).opacity = isSelected ? 0.8 : 0.2;

        // Draw beam to the corresponding Ground Station
        const gsLoc = gsLocations.find(gs => gs.id === sat.linkedGsId);
        if (gsLoc) {
          // Calculate current rotated position of ground station point
          const originalGsPos = latLonToVector(gsLoc.lat, gsLoc.lon, earthRadius);
          const rotatedGsPos = originalGsPos.clone().applyAxisAngle(new THREE.Vector3(0, 1, 0), earthGrid.rotation.y);
          
          // Set line geometry points
          const points = [satPos, rotatedGsPos];
          satMesh.line.geometry.setFromPoints(points);
          satMesh.line.computeLineDistances(); // Required for dashed lines

          // Modulate beam color/dash/opacity based on connection from ref
          const liveGs = currentGroundStations.find(g => g.station_id === sat.linkedGsId);
          const gsStatus = liveGs ? liveGs.status : 'ONLINE';
          
          let beamColor = 0x0ea5e9; // BLUE normal beam
          let opacity = isSelected ? 0.7 : 0.25;
          
          if (gsStatus === 'OUTAGE' || state === 'RED') {
            beamColor = 0xef4444; // RED interrupted beam
            opacity = isSelected ? 0.8 : 0.1;
          } else if (gsStatus === 'DEGRADED' || state === 'YELLOW') {
            beamColor = 0xf59e0b; // YELLOW degraded beam
          }

          (satMesh.line.material as THREE.LineDashedMaterial).color.setHex(beamColor);
          (satMesh.line.material as THREE.LineDashedMaterial).opacity = opacity;
        }
      });

      // Slowly rotate overall scene for a cinematic view
      scene.rotation.y = elapsed * 0.02;

      renderer.render(scene, camera);
    };

    animate();

    // 8. Resize handler
    const handleResize = () => {
      const w = container.clientWidth;
      const h = container.clientHeight;
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      renderer.setSize(w, h);
    };

    window.addEventListener('resize', handleResize);

    // 9. Cleanup
    return () => {
      window.removeEventListener('resize', handleResize);
      cancelAnimationFrame(animationId);
      if (container && renderer.domElement) {
        container.removeChild(renderer.domElement);
      }
      
      // Traverse scene and clean up Three.js resources
      scene.traverse((object) => {
        if (object instanceof THREE.Mesh) {
          object.geometry.dispose();
          if (Array.isArray(object.material)) {
            object.material.forEach((mat) => mat.dispose());
          } else {
            object.material.dispose();
          }
        } else if (object instanceof THREE.Line) {
          object.geometry.dispose();
          if (Array.isArray(object.material)) {
            object.material.forEach((mat) => mat.dispose());
          } else {
            object.material.dispose();
          }
        }
      });
      renderer.dispose();
    };
  }, []); // Re-run effect only on mount / unmount

  return (
    <div className="glass-panel" style={{ height: '100%', position: 'relative' }}>
      <div className="panel-header">
        <span>Orbital Coverage Network</span>
        <div style={{ display: 'flex', gap: '10px', fontSize: '11px' }}>
          <span style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
            <span className="status-indicator green" style={{ width: '6px', height: '6px' }} /> Healthy
          </span>
          <span style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
            <span className="status-indicator yellow" style={{ width: '6px', height: '6px' }} /> Degraded
          </span>
          <span style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
            <span className="status-indicator red" style={{ width: '6px', height: '6px' }} /> Outage
          </span>
        </div>
      </div>
      <div ref={mountRef} style={{ width: '100%', height: 'calc(100% - 38px)', cursor: 'grab' }} />
      <div
        style={{
          position: 'absolute',
          bottom: '12px',
          left: '12px',
          pointerEvents: 'none',
          fontSize: '11px',
          color: 'var(--text-secondary)',
          background: 'rgba(7, 9, 14, 0.7)',
          padding: '6px 10px',
          borderRadius: '4px',
          fontFamily: 'var(--font-mono)'
        }}
      >
        TELEMETRY RECEIVER: ACTIVE<br />
        POLAR COORDS RENDERER: V3D-EARTH
      </div>
    </div>
  );
};
