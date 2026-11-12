"""Generate RaceDesk's original short three-note Race Ready reminder."""
import math
import struct
import wave
from pathlib import Path

rate = 44100
length = 1.55
notes = [(0.05, 660.0, 0.19), (0.36, 660.0, 0.19), (0.69, 990.0, 0.55)]
samples = []
for index in range(int(rate * length)):
    time = index / rate
    value = 0.0
    for start, frequency, duration in notes:
        t = time - start
        if 0 <= t < duration:
            envelope = min(1.0, t / 0.012) * min(1.0, (duration - t) / 0.06) * math.exp(-2.4 * t)
            value += 0.5 * envelope * (math.sin(2 * math.pi * frequency * t) + 0.18 * math.sin(4 * math.pi * frequency * t))
    samples.append(int(max(-1.0, min(1.0, value)) * 32767))
with wave.open(str(Path(__file__).with_name('race-ready.wav')), 'wb') as out:
    out.setnchannels(1)
    out.setsampwidth(2)
    out.setframerate(rate)
    out.writeframes(struct.pack('<' + 'h' * len(samples), *samples))
