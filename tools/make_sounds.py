"""Generate the small, original arcade sound effects used by both versions."""

from __future__ import annotations

import math
import wave
from array import array
from pathlib import Path

RATE = 22050
OUT = Path(__file__).resolve().parents[1] / "assets"


def write(name: str, seconds: float, synth) -> None:
    samples = array("h")
    for i in range(int(RATE * seconds)):
        t = i / RATE
        value = max(-1.0, min(1.0, synth(t, seconds)))
        samples.append(int(value * 16000))
    OUT.mkdir(exist_ok=True)
    with wave.open(str(OUT / f"{name}.wav"), "wb") as file:
        file.setnchannels(1)
        file.setsampwidth(2)
        file.setframerate(RATE)
        file.writeframes(samples.tobytes())


def chirp(t: float, length: float, start: float, end: float) -> float:
    phase = 2 * math.pi * (start * t + (end - start) * t * t / (2 * length))
    return math.sin(phase)


def fade(t: float, length: float) -> float:
    return max(0.0, 1.0 - t / length) ** 1.7


def main() -> None:
    write("laser", .13, lambda t, n: .55 * fade(t, n) * (chirp(t, n, 920, 290) + .25 * chirp(t, n, 1840, 580)))
    write("hit", .24, lambda t, n: .52 * fade(t, n) * (chirp(t, n, 420, 1050) + .3 * math.sin(2 * math.pi * 1300 * t)))
    write("damage", .33, lambda t, n: .55 * fade(t, n) * (chirp(t, n, 190, 58) + .2 * math.sin(2 * math.pi * 37 * t)))
    write("warning", .27, lambda t, n: .43 * (1 if t < .10 or .15 < t < .25 else 0) * math.sin(2 * math.pi * 740 * t))

    def fanfare(t: float, n: float) -> float:
        notes = (523.25, 659.25, 783.99, 1046.5)
        note = notes[min(3, int(t / (n / 4)))]
        return .40 * fade(t, n) * (math.sin(2 * math.pi * note * t) + .25 * math.sin(4 * math.pi * note * t))

    write("victory", .85, fanfare)
    write("defeat", .55, lambda t, n: .48 * fade(t, n) * chirp(t, n, 440, 100))


if __name__ == "__main__":
    main()
