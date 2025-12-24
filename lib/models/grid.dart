enum GridValue {
  full(1),
  half(2),
  third(3),
  quarter(4),
  sixth(6),
  eighth(8),
  sixteenth(16),
  thirtysecond(32),
  sixtyfourth(64);

  final int value;
  const GridValue(this.value);
}
