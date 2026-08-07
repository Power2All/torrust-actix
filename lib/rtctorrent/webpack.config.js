const path = require('path');
const webpack = require('webpack');

module.exports = [
  {
    name: 'browser',
    entry: './src/rtctorrent.js',
    output: {
      filename: 'rtctorrent.browser.js',
      path: path.resolve(__dirname, 'dist'),
      library: {
        name: 'RtcTorrent',
        type: 'umd',
        export: 'default'
      },
      globalObject: 'this'
    },
    target: 'web',
    module: {
      rules: [
        {
          test: /\.js$/,
          exclude: /node_modules/,
          use: {
            loader: 'babel-loader',
            options: {
              presets: ['@babel/preset-env']
            }
          }
        }
      ]
    },
    resolve: {
      extensions: ['.js'],
      // Every require() of a Node core module in src/ sits behind an
      // isBrowser / crypto.subtle / typeof fetch guard, so the browser bundle
      // never reaches one. Stubbing them out instead of polyfilling keeps
      // crypto-browserify -> elliptic (and the rest of node-stdlib-browser)
      // out of the shipped bundle entirely. Only the Buffer global is real.
      fallback: {
        "fs": false,
        "path": false,
        "crypto": false,
        "stream": false,
        "http": false,
        "https": false,
        "url": false,
        "buffer": require.resolve("buffer/")
      }
    },
    plugins: [
      new webpack.ProvidePlugin({ Buffer: ['buffer', 'Buffer'] })
    ],
    mode: 'production',
    devtool: 'source-map'
  },
  {
    name: 'node',
    entry: './src/rtctorrent.js',
    output: {
      filename: 'rtctorrent.node.js',
      path: path.resolve(__dirname, 'dist'),
      library: {
        name: 'RtcTorrent',
        type: 'umd',
        export: 'default'
      },
      globalObject: 'this'
    },
    target: 'node',
    externals: {
      'wrtc': 'commonjs wrtc',
      '@roamhq/wrtc': 'commonjs @roamhq/wrtc',
      'node-webrtc': 'commonjs node-webrtc'
    },
    module: {
      rules: [
        {
          test: /\.js$/,
          exclude: /node_modules/,
          use: {
            loader: 'babel-loader',
            options: {
              presets: ['@babel/preset-env']
            }
          }
        }
      ]
    },
    mode: 'production',
    devtool: 'source-map'
  }
];