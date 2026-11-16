import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import './styles.css';

function App() {
  return <main><p className="eyebrow">MEET AGAIN / 江苏</p>
    <h1>各自生活，<br/>还会再相逢吗？</h1>
    <p className="intro">十三座城市。一张道路网络。观察独立生活的轨迹，何时再次交汇。</p>
    <div className="status"><span aria-hidden="true">◌</span> 模拟引擎建设中</div>
    <footer>工程骨架已建立 · 交互界面将在地图与 API 就绪后开放</footer>
  </main>;
}

createRoot(document.getElementById('root')!).render(<StrictMode><App/></StrictMode>);
