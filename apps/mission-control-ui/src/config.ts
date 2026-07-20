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

const getBackendUrl = (envVal: string | undefined, defaultPort: string) => {
  if (envVal) return envVal;
  if (typeof window !== 'undefined') {
    const { protocol, hostname } = window.location;
    return `${protocol}//${hostname}:${defaultPort}`;
  }
  return `http://localhost:${defaultPort}`;
};

export const API_CONFIG = {
  REPLAY_URL: getBackendUrl(import.meta.env.VITE_REPLAY_API_URL, '8084'),
  COLLECTOR_URL: getBackendUrl(import.meta.env.VITE_COLLECTOR_API_URL, '8080'),
  COMMAND_URL: getBackendUrl(import.meta.env.VITE_COMMAND_API_URL, '8085'),
};
