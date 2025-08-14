
## App View and App Compoents
- App View and App Components are VanillaJS Web Components. Each Uses litHTML library to render to the DOM. Simply extends HTMLElement Class
- DOM rendering is done by litHTML, so we make sure to adhere to litHTML rendering best practises.
- Both App Views and App Components are just Web Components. 
- An App View is essentially a single page within an app and corresponds to a URL, e.g. example.com/v/home or example.com/v/todo_items
- An App Component is any encapsulation of logic or UI utilized by an App View, e.g. 'custom-button' or 'custom-input'
- All App Views and App Components are contained within a single directory. 
    - Each directory has 3 files: ts, html and css that encapsulte the logic, view and style of the component or view. 
    - All three files (ts, html and css) are bundled as one js file in build process. 
    - Always put string literal html content (for litHTML rendering) within the html file.   
    - views and components are contained within lazy/views and lazy/components respectively, e.g. ./lazy/views/example_view and ./lazy/components/example_component
    - ts, html and css files are named the same as directory, e.g. lazy/components/button/button.ts, lazy/components/button/button.html, lazy/components/button/button.css
- Each App View and App Component contains an AttributeT, ModelT, and StateT for: attributes on element in dom; data from server that is immutable; and state data which is mutable local state

### App View Specifics
- Each App View must call a framework utilty called ViewConnectedCallback which sets up baseline functionality, handles initial data load and render.
- Each App View must dispatchEvent 'hydrated' within the connectedCallback to specify that the DOM is hydrated and rendered and ready to be displayed
- Each App View must contain an sc function (short for statechanged). It renders (via litHTML) the attribute, model and state to the DOM
- Each View must contain a kd function (short for knitdata). This function is called when data is loaded or updated from the server. It is responsible for knitting and morphing the server data into usable data for view's model. kd only runs on view instantiation or if server sends new modified data to client.



