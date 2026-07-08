import { BrowserRouter, Routes, Route } from 'react-router-dom';
import Home from './pages/Home';
import Manifest from './pages/Manifest';
import BuildLog from './pages/BuildLog';
import Decisions from './pages/Decisions';
import Outcomes from './pages/Outcomes';
import { AppProviders } from './providers/AppProviders';
import { Layout } from './components/Layout';

export default function App() {
  return (
    <AppProviders>
      <BrowserRouter>
        <Routes>
          <Route element={<Layout />}>
            <Route path="/" element={<Home />} />
            <Route path="/manifest" element={<Manifest />} />
            <Route path="/build-log" element={<BuildLog />} />
            <Route path="/decisions" element={<Decisions />} />
            <Route path="/outcomes" element={<Outcomes />} />
          </Route>
        </Routes>
      </BrowserRouter>
    </AppProviders>
  );
}
