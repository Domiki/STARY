import { useViewerStore } from '../../viewer/store'
import { HistogramSection } from './HistogramSection'
import { PropertiesSection } from './PropertiesSection'

export function InspectorPanel() {
  const histogram = useViewerStore((state) => state.view?.histogram ?? null)
  return (
    <div className="min-h-0 flex-1 overflow-auto">
      <HistogramSection histogram={histogram} />
      <PropertiesSection />
    </div>
  )
}
