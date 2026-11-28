// AudioWorklet: plays the samples the game (in the worker) renders into a
// shared ring buffer of interleaved stereo frames. Frame counters wrap
// around (the ring size is a power of two).
class KeenAudio extends AudioWorkletProcessor {
  constructor(options) {
    super();
    const { sab, L, ahead } = options.processorOptions;
    this.hdr = new Int32Array(sab, 0, 16);
    this.samples = new Float32Array(sab, L.A_SAMPLES, L.A_FRAMES * 2);
    this.mask = L.A_FRAMES - 1;
    this.L = L;
    this.ahead = ahead;
  }

  process(_inputs, outputs) {
    const out = outputs[0];
    const left = out[0];
    const right = out[1] || out[0];
    const n = left.length;
    const { hdr, samples, mask, L, ahead } = this;
    const write = Atomics.load(hdr, L.A_WRITE);
    let read = Atomics.load(hdr, L.A_READ);
    let avail = (write - read) | 0;
    // after a stall (or the context was suspended) or when the clocks
    // drifted apart: back to the usual latency
    if (avail < 0 || avail > 2 * ahead) {
      read = (write - ahead) | 0;
      avail = ahead;
    }
    const m = Math.min(n, avail);
    for (let i = 0; i < m; i++) {
      const k = ((read + i) & mask) * 2;
      left[i] = samples[k];
      right[i] = samples[k + 1];
    }
    for (let i = m; i < n; i++) {
      left[i] = 0;
      right[i] = 0;
    }
    Atomics.store(hdr, L.A_READ, (read + m) | 0);
    return true;
  }
}

registerProcessor('keen-audio', KeenAudio);
