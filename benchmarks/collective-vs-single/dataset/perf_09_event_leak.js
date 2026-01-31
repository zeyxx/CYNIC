// React component with event listener
import { useEffect, useState } from 'react';

function WindowSizeTracker() {
  const [size, setSize] = useState({ width: 0, height: 0 });

  useEffect(() => {
    const handleResize = () => {
      setSize({ width: window.innerWidth, height: window.innerHeight });
    };

    window.addEventListener('resize', handleResize);
    // Missing: return () => window.removeEventListener('resize', handleResize);
  }, []);

  return <div>Window: {size.width}x{size.height}</div>;
}

export default WindowSizeTracker;
