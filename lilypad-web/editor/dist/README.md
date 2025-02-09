# Lilypad

A text-based visual code editor architected to supported multiple languages and platforms.

## Use in HTML Files

- include the following html tag in files to insert a Lilypad web editor:
```
  <script src="./node_modules/lilypad-web-editor/editor.js"></script>
```

## Use in PreText
- place the package in the output/external/ directory
- note that the script element referencing the editor needs to be assoicated with a canvas element having a "lilypad-canvas" id
- it is helpful to use a top level div element to declare the namespace 

```
<interactive aspect="1:1" width="100%" xml:id="lilly-1" platform="javascript">
  <slate surface="html">
    <div xmlns="http://www.w3.org/1999/xhtml" style="width: 100%; height: 100%">
      <canvas id="lilypad-canvas" style="width: 100%; height: 98%;"></canvas>
      <script src="external/lilypad-web-editor/editor.js"></script>
    </div>
  </slate>
</interactive>
```

- see the following repository for a more detailed example: https://github.com/SophieRuetschi/Lilypad-PreText