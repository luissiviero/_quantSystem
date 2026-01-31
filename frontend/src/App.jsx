// File: src/App.jsx
import React from 'react';
import { Layout, Model } from 'flexlayout-react';
import 'flexlayout-react/style/dark.css'; 

// Import Components
import Header from './components/Header';
import { ChartComponent } from './components/ChartComponent';
import OrderBook from './components/OrderBook';
import TradesComponent from './components/TradesComponent';
import { OrderForm } from './components/OrderForm';
import layoutJson from './layout.json';

// Footer Component
const Footer = React.memo(() => (
  <div style={{ height: '25px', backgroundColor: '#161a25', borderTop: '1px solid #0b0e11', display: 'flex', alignItems: 'center', padding: '0 10px', fontSize: '11px', color: '#848e9c', flexShrink: 0 }}>
    <span style={{ display: 'flex', alignItems: 'center', gap: '5px', color: '#0ecb81' }}>
      <div style={{ width: '6px', height: '6px', borderRadius: '50%', background: '#0ecb81' }}></div> Stable Connection
    </span>
  </div>
));

const model = Model.fromJson(layoutJson);

function App() {
  const factory = (node) => {
    const component = node.getComponent();
    
    // Wire up the new optimized components
    if (component === 'chart') return <ChartComponent />;
    if (component === 'orderBook') return <OrderBook />;
    if (component === 'orderForm') return <OrderForm />;
    if (component === 'trades') return <TradesComponent />;
    
    // Placeholder for empty tabs
    if (component === 'positions' || component === 'history' || component === 'assets') {
      return (
        <div className="panel-content" style={{alignItems: 'center', justifyContent: 'center', color: '#474d57', fontSize: '12px'}}>
          {node.getName()} - No Data
        </div>
      );
    }
    
    return <div className="panel-content">Panel: {component}</div>;
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh', width: '100vw' }}>
      <Header />
      <div style={{ flex: 1, position: 'relative' }}>
        <Layout model={model} factory={factory} />
      </div>
      <Footer />
    </div>
  );
}

export default App;