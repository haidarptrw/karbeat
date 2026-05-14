import 'package:flutter/material.dart';

final List<Color> dawColors = [
      Colors.redAccent, Colors.pinkAccent, Colors.purpleAccent, Colors.deepPurpleAccent,
      Colors.indigoAccent, Colors.blueAccent, Colors.lightBlueAccent, Colors.cyanAccent,
      Colors.tealAccent, Colors.greenAccent, Colors.lightGreenAccent, Colors.limeAccent,
      Colors.yellowAccent, Colors.amberAccent, Colors.orangeAccent, Colors.deepOrangeAccent,
      Colors.brown.shade400, Colors.grey.shade400, Colors.blueGrey.shade400, Colors.white70,
    ];

extension HexColorParsing on String {
  /// Converts a Rust/Web hex color string (#RRGGBB or #RRGGBBAA) to a Flutter Color
  Color toColor() {
    // 1. Remove the '#' if it exists
    String hex = replaceAll('#', '');

    // 2. Handle standard 6-char hex (RRGGBB) by forcing 100% opacity (FF)
    if (hex.length == 6) {
      hex = 'FF$hex';
    } 
    // 3. Handle 8-char hex (RRGGBBAA) by moving AA from the end to the front (AARRGGBB)
    else if (hex.length == 8) {
      hex = '${hex.substring(6, 8)}${hex.substring(0, 6)}';
    } 
    // Fallback for invalid strings
    else {
      return Colors.grey; 
    }

    // 4. Parse it as a radix 16 integer
    return Color(int.parse(hex, radix: 16));
  }
}

extension HexStringCast on Color {
  /// transform the flutter Color type into RGGBA hex string representation
  String toRGBA() {
    final r = (this.r * 255).round().toRadixString(16).padLeft(2, '0');
    final g = (this.g * 255).round().toRadixString(16).padLeft(2, '0');
    final b = (this.b * 255).round().toRadixString(16).padLeft(2, '0');
    final a = (this.a * 255).round().toRadixString(16).padLeft(2, '0');

    return '$r$g$b$a'.toUpperCase();
  } 
}