// File: src/services/BinanceStream.js
export class BinanceStream {
  constructor() {
    this.ws = null;
    this.subscribers = new Map(); // Key: streamName, Value: Set of callbacks
    this.isConnected = false;
    this.queue = []; 
    this.baseUrl = 'wss://fstream.binance.com/ws'; 
  }

  connect() {
    if (this.ws) return;

    this.ws = new WebSocket(this.baseUrl);

    this.ws.onopen = () => {
      console.log('[BinanceStream] Connected');
      this.isConnected = true;
      this.processQueue();
    };

    this.ws.onmessage = (event) => {
      const msg = JSON.parse(event.data);
      
      // If it's a control message (like "result": null), ignore it
      if (!msg.e && !msg.data) return;

      const symbol = msg.s ? msg.s.toLowerCase() : '';
      let streamKey = null;

      // --- CRITICAL FIX: Mapping Event Types to Subscription Keys ---
      
      if (msg.e === '24hrTicker') {
        // Map 24hrTicker event -> @ticker subscription
        streamKey = `${symbol}@ticker`;
      } 
      else if (msg.e === 'aggTrade') {
        streamKey = `${symbol}@aggTrade`;
      } 
      else if (msg.e === 'depthUpdate') {
        // Map depthUpdate event -> @depth10@100ms subscription
        // Note: We assume we are only using this specific depth stream for this app
        streamKey = `${symbol}@depth10@100ms`;
      } 
      else if (msg.e === 'kline') {
        streamKey = `${symbol}@kline_${msg.k.i}`;
      }

      // Dispatch data to subscribers
      if (streamKey && this.subscribers.has(streamKey)) {
        this.subscribers.get(streamKey).forEach(cb => cb(msg));
      }
    };

    this.ws.onclose = () => {
      console.log('[BinanceStream] Disconnected. Reconnecting...');
      this.isConnected = false;
      this.ws = null;
      setTimeout(() => this.connect(), 3000);
    };

    this.ws.onerror = (err) => {
      console.error('[BinanceStream] Error', err);
    };
  }

  subscribe(stream, callback) {
    if (!this.subscribers.has(stream)) {
      this.subscribers.set(stream, new Set());
      this.send('SUBSCRIBE', [stream]);
    }
    this.subscribers.get(stream).add(callback);
  }

  unsubscribe(stream, callback) {
    if (this.subscribers.has(stream)) {
      const set = this.subscribers.get(stream);
      set.delete(callback);
      if (set.size === 0) {
        this.subscribers.delete(stream);
        this.send('UNSUBSCRIBE', [stream]);
      }
    }
  }

  send(method, params) {
    const payload = JSON.stringify({ method, params, id: Date.now() });
    if (this.isConnected) {
      this.ws.send(payload);
    } else {
      this.queue.push(payload);
    }
  }

  processQueue() {
    while (this.queue.length > 0) {
      this.ws.send(this.queue.shift());
    }
  }
}

export const binanceStream = new BinanceStream();