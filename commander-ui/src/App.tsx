import { Button } from '@kobalte/core'
import { invoke } from '@tauri-apps/api/tauri'
import {
  createContext,
  createResource,
  createSignal,
  Show,
  useContext,
  type Component
} from 'solid-js'

type DirectoryContextType = [
  () => string,
  { set: (value: string) => void; pop: () => void }
]

const DirectoryContext = createContext<DirectoryContextType>()

const App: Component = () => {
  const [directory, setDirectory] = createSignal(
    '/Users/keatonbrandt/Documents/Development'
  )
  const directoryContext: DirectoryContextType = [
    directory,
    {
      set: function (value: string) {
        setDirectory(value)
      },
      pop: function () {
        const split = directory().split('/')
        split.pop()
        setDirectory(split.join('/'))
      }
    }
  ]

  return (
    <div class="window">
      <DirectoryContext.Provider value={directoryContext}>
        <header>
          <h1 class="text-xl font-extrabold">Commander</h1>
        </header>
        <main>
          <div class="program-container">
            <section>
              <DirectoryPicker />
            </section>
          </div>
        </main>
      </DirectoryContext.Provider>
    </div>
  )
}

type FileEntityDetails = {name: string, path: string};
type FileEntity = { File: FileEntityDetails } | { Directory: FileEntityDetails }

const DirectoryPicker: Component = () => {
  const [directory, { set, pop }] = useContext(DirectoryContext)
  const [entities] = createResource<FileEntity[]>(directory, d =>
    invoke('list_files_in_relative_directory', { path: d })
  )

  return (
    <ol>
      <li>
        <Button.Root onClick={() => pop()}>..</Button.Root>
      </li>
      <For each={entities()} fallback={<div>Loading...</div>}>
        {(entity: FileEntity) => (
          <Show when={'Directory' in entity}>
            <li>
              <Button.Root onClick={() => set(entity.Directory.path)}>
                {entity.Directory.name}
              </Button.Root>
            </li>
          </Show>
        )}
      </For>
    </ol>
  )
}

export default App
