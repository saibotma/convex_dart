import 'dart:async';
import 'dart:convert';

import 'rust/api/convex_client.dart' as rust;
import 'rust/api/subscription.dart' as rust_sub;

/// A Dart-idiomatic wrapper for the Convex client.
class ConvexClient {
  final rust.ConvexClientWrapper _client;
  bool _isConnected = false;

  ConvexClient._() : _client = rust.ConvexClientWrapper();

  /// Creates a new ConvexClient instance.
  static ConvexClient create() {
    return ConvexClient._();
  }

  /// Connects to a Convex deployment.
  Future<void> connect(String deploymentUrl) async {
    try {
      await _client.connect(deploymentUrl: deploymentUrl);
      _isConnected = true;
    } on rust.ConvexError catch (e) {
      throw ConvexException(e.message);
    }
  }

  /// Returns true if the client is connected to a deployment.
  bool get isConnected => _isConnected;

  /// Performs a mutation on the Convex backend.
  Future<T> mutation<T>(String functionName, [Map<String, dynamic>? args]) async {
    if (!_isConnected) {
      throw ConvexException('Client not connected. Call connect() first.');
    }

    final convexArgs = _convertArgsToConvexValues(args ?? {});
    
    try {
      final result = await _client.mutation(
        functionName: functionName,
        args: convexArgs,
      );
      return _parseResult<T>(result);
    } on rust.ConvexError catch (e) {
      throw ConvexException(e.message);
    }
  }

  /// Performs a query on the Convex backend.
  Future<T> query<T>(String functionName, [Map<String, dynamic>? args]) async {
    if (!_isConnected) {
      throw ConvexException('Client not connected. Call connect() first.');
    }

    final convexArgs = _convertArgsToConvexValues(args ?? {});
    
    try {
      final result = await _client.query(
        functionName: functionName,
        args: convexArgs,
      );
      return _parseResult<T>(result);
    } on rust.ConvexError catch (e) {
      throw ConvexException(e.message);
    }
  }

  /// Creates a subscription to a query that returns a stream of results.
  Stream<T> subscribe<T>(String functionName, [Map<String, dynamic>? args]) async* {
    if (!_isConnected) {
      throw ConvexException('Client not connected. Call connect() first.');
    }

    final convexArgs = _convertArgsToConvexValues(args ?? {});
    
    try {
      final subscription = await _client.subscribe(
        functionName: functionName,
        args: convexArgs,
      );

      yield* _createReactiveStream<T>(subscription);
    } on rust.ConvexError catch (e) {
      throw ConvexException(e.message);
    }
  }

  Stream<T> _createReactiveStream<T>(rust_sub.ConvexSubscription subscription) async* {
    try {
      while (true) {
        final next = await subscription.next();
        if (next == null) {
          // Subscription ended naturally
          break;
        }
        
        final result = _parseResult<T>(next);
        yield result;
      }
    } catch (e) {
      throw ConvexException('Subscription error: $e');
    }
  }

  List<(String, rust.ConvexValue)> _convertArgsToConvexValues(Map<String, dynamic> args) {
    return args.entries.map((entry) {
      final value = entry.value;
      final convexValue = _dartValueToConvexValue(value);
      return (entry.key, convexValue);
    }).toList();
  }

  rust.ConvexValue _dartValueToConvexValue(dynamic value) {
    if (value == null) {
      return rust.ConvexValue.null_();
    } else if (value is String) {
      return rust.ConvexValue.fromString(value: value);
    } else if (value is int) {
      return rust.ConvexValue.fromInt(value: value);
    } else if (value is double) {
      return rust.ConvexValue.fromDouble(value: value);
    } else if (value is bool) {
      return rust.ConvexValue.fromBool(value: value);
    } else {
      // For complex types, serialize to JSON string
      final jsonString = jsonEncode(value);
      return rust.ConvexValue(inner: jsonString);
    }
  }

  T _parseResult<T>(rust.ConvexValue result) {
    final jsonString = result.inner;
    final parsed = jsonDecode(jsonString);
    return parsed as T;
  }
}

/// Exception thrown by Convex operations.
class ConvexException implements Exception {
  final String message;
  
  const ConvexException(this.message);
  
  @override
  String toString() => 'ConvexException: $message';
}