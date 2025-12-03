import 'package:flutter/material.dart';
import 'package:karbeat/features/screens/main_screen.dart';

class KarbeatApp extends StatelessWidget {
  const KarbeatApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Karbeat DAW',
      theme: ThemeData.dark(),
      home: const MainScreen(),
    );
  }
}