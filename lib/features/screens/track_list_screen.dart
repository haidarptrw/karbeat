import 'package:flutter/material.dart';

class TrackListScreen extends StatelessWidget {
  TrackListScreen({super.key});

  // TEMPORARY for placeholder
  final List<String> tracks = List.empty();

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final parentHeight = constraints.maxHeight;

        // Safety check: If for some reason height is still infinite (unlikely with Expanded)
        if (parentHeight.isInfinite) return const SizedBox();

        // Dynamic height: 20% of screen, but at least 80px, max 150px
        final calculatedHeight = parentHeight * 0.20;
        final double itemHeight = calculatedHeight.clamp(80.0, 150.0);

        return ListView.builder(
          padding: EdgeInsets.zero,
          itemCount: tracks.length + 1, // +1 for Add Track button
          itemBuilder: (context, index) {
            // ========== ADD BUTTON (Last Item) ==========
            if (index == tracks.length) {
              return SizedBox(
                height: 60, // Smaller height for the button
                child: Center(
                  child: TextButton.icon(
                    onPressed: () {
                      // Call your Add Track logic here
                      // context.read<KarbeatState>().addTrack();
                    },
                    icon: const Icon(Icons.add, color: Colors.white54),
                    label: const Text(
                      "Add New Track",
                      style: TextStyle(color: Colors.white54),
                    ),
                  ),
                ),
              );
            }

            // ========== TRACK ITEM ==========
            return SizedBox(
              height: itemHeight,
              child: Container(
                margin: const EdgeInsets.only(bottom: 2),
                padding: const EdgeInsets.symmetric(horizontal: 10),
                decoration: BoxDecoration(
                  color: Colors.grey.shade300,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Row(
                      children: [
                        // Track Name
                        Expanded(
                          child: Text(
                            "Track ${index + 1}", 
                            style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold)
                          ),
                        ),
                      ],
                    ),
              ),
            );
          },
        );
      },
    );
  }
}
