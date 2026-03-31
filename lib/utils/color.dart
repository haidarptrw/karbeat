import 'package:flutter/material.dart';

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