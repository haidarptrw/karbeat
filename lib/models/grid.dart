enum GridSize {
  oneBar(0.25),
  twoBeat(0.5),
  full(1),
  half(2),
  third(3),
  quarter(4),
  sixth(6),
  eighth(8),
  twelfth(12),
  sixteenth(16),
  thirtysecond(32),
  sixtyfourth(64),
  infinity(double.infinity);

  final double value;
  const GridSize(this.value);
}

enum MusicalBeatSize {
  four(4),
  three(3),
  two(2),
  one(1),
  half(0.5),
  third(1 / 3),
  quarter(1 / 4),
  sixth(1 / 6),
  eighth(1 / 8),
  twelfth(1 / 12),
  sixteenth(1 / 16),
  thirtysecond(1 / 32),
  sixtyfourth(1 / 64),
  none(0); // this means one tick size

  final double value;
  const MusicalBeatSize(this.value);
}
