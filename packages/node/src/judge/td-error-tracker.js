// TD Error Tracker
'use strict';

import { getPool } from '@cynic/persistence';
import { globalEventBus, EventType } from '@cynic/core';

const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;

export const TD_CONFIG = {
  windowSize: 100,
  convergenceThreshold: 0.1,
  driftThreshold: 0.3,
  minUpdatesForConvergence: 50,
  alertCooldownMs: 30 * 60 * 1000,
};
