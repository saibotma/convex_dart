import 'package:flutter_test/flutter_test.dart';
import 'package:convex_dart/convex_dart.dart';

void main() {
  setUpAll(() async {
    await RustLib.init();
  });

  group('ConvexClient', () {
    test('should create client successfully', () {
      final client = ConvexClient.create();
      expect(client, isNotNull);
      expect(client.isConnected, isFalse);
    });

    test('should throw exception when not connected', () async {
      final client = ConvexClient.create();
      
      expect(
        () => client.query('test'),
        throwsA(isA<ConvexException>()),
      );
      
      expect(
        () => client.mutation('test'),
        throwsA(isA<ConvexException>()),
      );
    });

    test('should handle connect with invalid URL', () async {
      final client = ConvexClient.create();
      
      expect(
        () => client.connect('invalid-url'),
        throwsA(isA<ConvexException>()),
      );
    });
  });

  group('ConvexException', () {
    test('should create exception with message', () {
      const exception = ConvexException('Test message');
      expect(exception.message, equals('Test message'));
      expect(exception.toString(), equals('ConvexException: Test message'));
    });
  });
}