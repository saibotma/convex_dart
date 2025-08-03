import 'package:flutter/material.dart';
import 'package:convex_dart/convex_dart.dart';

void main() async {
  // Initialize the Rust library
  await RustLib.init();
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Convex Dart Demo',
      theme: ThemeData(
        primarySwatch: Colors.blue,
      ),
      home: const ConvexDemo(),
    );
  }
}

class ConvexDemo extends StatefulWidget {
  const ConvexDemo({super.key});

  @override
  State<ConvexDemo> createState() => _ConvexDemoState();
}

class _ConvexDemoState extends State<ConvexDemo> {
  late ConvexClient _client;
  bool _isConnected = false;
  String _status = 'Not connected';
  final _urlController = TextEditingController();

  @override
  void initState() {
    super.initState();
    _client = ConvexClient.create();
  }

  Future<void> _connect() async {
    try {
      await _client.connect(_urlController.text);
      setState(() {
        _isConnected = true;
        _status = 'Connected to ${_urlController.text}';
      });
    } on ConvexException catch (e) {
      setState(() {
        _status = 'Error: ${e.message}';
      });
    }
  }

  Future<void> _runQuery() async {
    try {
      // Example query - replace with your actual Convex function name
      final result = await _client.query('listMessages');
      setState(() {
        _status = 'Query result: $result';
      });
    } on ConvexException catch (e) {
      setState(() {
        _status = 'Query error: ${e.message}';
      });
    }
  }

  Future<void> _runMutation() async {
    try {
      // Example mutation - replace with your actual Convex function name
      final result = await _client.mutation('sendMessage', {
        'body': 'Hello from Dart!',
        'author': 'Flutter App',
      });
      setState(() {
        _status = 'Mutation result: $result';
      });
    } on ConvexException catch (e) {
      setState(() {
        _status = 'Mutation error: ${e.message}';
      });
    }
  }

  void _subscribeToQuery() {
    try {
      // Example subscription - replace with your actual Convex function name
      _client.subscribe('listMessages').listen(
        (data) {
          setState(() {
            _status = 'Subscription update: $data';
          });
        },
        onError: (error) {
          setState(() {
            _status = 'Subscription error: $error';
          });
        },
      );
    } on ConvexException catch (e) {
      setState(() {
        _status = 'Subscription error: ${e.message}';
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Convex Dart Demo'),
      ),
      body: Padding(
        padding: const EdgeInsets.all(16.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            TextField(
              controller: _urlController,
              decoration: const InputDecoration(
                labelText: 'Convex Deployment URL',
                hintText: 'https://your-project.convex.cloud',
              ),
            ),
            const SizedBox(height: 16),
            ElevatedButton(
              onPressed: _isConnected ? null : _connect,
              child: const Text('Connect'),
            ),
            const SizedBox(height: 16),
            ElevatedButton(
              onPressed: _isConnected ? _runQuery : null,
              child: const Text('Run Query'),
            ),
            const SizedBox(height: 8),
            ElevatedButton(
              onPressed: _isConnected ? _runMutation : null,
              child: const Text('Run Mutation'),
            ),
            const SizedBox(height: 8),
            ElevatedButton(
              onPressed: _isConnected ? _subscribeToQuery : null,
              child: const Text('Subscribe to Query'),
            ),
            const SizedBox(height: 16),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                border: Border.all(color: Colors.grey),
                borderRadius: BorderRadius.circular(4),
              ),
              child: Text(
                'Status: $_status',
                style: const TextStyle(fontFamily: 'monospace'),
              ),
            ),
          ],
        ),
      ),
    );
  }

  @override
  void dispose() {
    _urlController.dispose();
    super.dispose();
  }
}
