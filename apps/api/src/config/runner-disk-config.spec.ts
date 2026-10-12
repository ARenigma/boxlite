/*
 * Copyright 2026 BoxLite AI
 * SPDX-License-Identifier: AGPL-3.0
 */

import { runnerDiskConfig } from './configuration'

describe('runnerDiskConfig', () => {
  it('defaults to no overcommit, no expand limit and a 10% free-space floor', () => {
    expect(runnerDiskConfig({})).toEqual({ overcommitRatio: 1, expandLimitGiB: undefined, minFreePercentage: 10 })
  })

  it('reads valid overrides', () => {
    expect(
      runnerDiskConfig({
        RUNNER_DISK_OVERCOMMIT_RATIO: '1.5',
        RUNNER_DISK_EXPAND_LIMIT_GIB: '4096',
        RUNNER_DISK_MIN_FREE_PERCENTAGE: '0',
      }),
    ).toEqual({ overcommitRatio: 1.5, expandLimitGiB: 4096, minFreePercentage: 0 })
  })

  // Each of these parses to a value that silently changes admission: NaN or 0
  // rejects every runner, a truncated limit admits almost nothing, and a
  // negative floor lets usage pass 100%.
  it.each([
    ['RUNNER_DISK_OVERCOMMIT_RATIO', 'abc'],
    ['RUNNER_DISK_OVERCOMMIT_RATIO', '0'],
    ['RUNNER_DISK_OVERCOMMIT_RATIO', '-1'],
    ['RUNNER_DISK_EXPAND_LIMIT_GIB', '1e9'],
    ['RUNNER_DISK_EXPAND_LIMIT_GIB', '0'],
    ['RUNNER_DISK_EXPAND_LIMIT_GIB', '4096.5'],
    ['RUNNER_DISK_MIN_FREE_PERCENTAGE', 'ten'],
    ['RUNNER_DISK_MIN_FREE_PERCENTAGE', '-10'],
    ['RUNNER_DISK_MIN_FREE_PERCENTAGE', '100'],
  ])('rejects %s=%s at boot', (name, value) => {
    expect(() => runnerDiskConfig({ [name]: value })).toThrow(name)
  })
})
