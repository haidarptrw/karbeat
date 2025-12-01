import 'package:flutter/material.dart';
import 'package:karbeat/features/main_screen.dart';

class KarbeatApp extends StatelessWidget {
  const KarbeatApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: MainScreen(),
    );
  }
}